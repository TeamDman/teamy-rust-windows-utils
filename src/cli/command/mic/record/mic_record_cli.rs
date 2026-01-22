use crate::cli::to_args::ToArgs;
use crate::services::{
    DrainAudioResult, FileCloseResult, FileOpenOptions, FileOpenResult, FileWriteResult,
    ServiceRuntime, StartRecordingResult, StopRecordingResult,
};
use arbitrary::Arbitrary;
use clap::Args;
use eyre::{Context, Result, bail};

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

/// Record audio from a microphone using roam-shm services.
///
/// This command demonstrates zero-copy audio data transfer using ShmBytes.
/// The architecture:
/// - MicrophoneService captures audio into shared memory
/// - FsService writes the data to disk from shared memory
/// - No data copying between services!
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct MicRecordArgs {
    /// The device ID to record from.
    #[clap(long)]
    pub id: String,

    /// Duration to record (e.g., "10s", "1m", "500ms").
    #[clap(long)]
    pub duration: String,

    /// Output file path for the WAV file.
    #[clap(long)]
    pub output_path: PathBuf,
}

impl MicRecordArgs {
    pub fn invoke(self) -> Result<()> {
        // Parse the duration
        let duration = humantime::parse_duration(&self.duration)
            .wrap_err_with(|| format!("Failed to parse duration: {}", self.duration))?;

        if duration.is_zero() {
            bail!("Duration must be greater than 0");
        }

        println!("🎙️  Starting roam-shm service runtime...");

        // Create tokio runtime
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .wrap_err("Failed to create tokio runtime")?;

        runtime.block_on(self.run_with_services(duration))
    }

    async fn run_with_services(self, duration: Duration) -> Result<()> {
        // Initialize the service runtime with roam-shm
        let services = ServiceRuntime::new().await?;

        println!("✅ Service runtime ready with ShmBytes support");
        println!(
            "📝 Recording from device {} for {:?}...",
            self.id, duration
        );

        // Start recording via MicrophoneService
        let start_result = services.mic.start_recording(self.id.clone()).await
            .wrap_err("RPC call to start_recording failed")?;

        match start_result {
            StartRecordingResult::Ok => {
                println!("🔴 Recording started");
            }
            StartRecordingResult::Err(e) => {
                bail!("Failed to start recording: {e}");
            }
        }

        // Wait for the recording duration
        tokio::time::sleep(duration).await;

        // Stop recording
        let stop_result = services.mic.stop_recording(self.id.clone()).await
            .wrap_err("RPC call to stop_recording failed")?;

        match stop_result {
            StopRecordingResult::Ok => {
                println!("⏹️  Recording stopped");
            }
            StopRecordingResult::Err(e) => {
                bail!("Failed to stop recording: {e}");
            }
        }

        // Give the recording thread time to finish
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Drain the audio to WAV (returns ShmBytes!)
        println!("📦 Draining audio to WAV (using ShmBytes)...");

        let drain_result = services.mic.drain_to_wav(self.id.clone()).await
            .wrap_err("RPC call to drain_to_wav failed")?;

        let audio_segment = match drain_result {
            DrainAudioResult::Ok(segment) => segment,
            DrainAudioResult::Err(e) => {
                bail!("Failed to drain audio: {e}");
            }
        };

        // Log audio info
        println!(
            "🎵 Audio segment: {}ms, {}Hz, {} channels, {} bits",
            audio_segment.duration_ms,
            audio_segment.sample_rate,
            audio_segment.channels,
            audio_segment.bits_per_sample
        );

        // len() doesn't need SHM context - it's stored in the struct
        let wav_size = audio_segment.bytes.len();
        println!("📊 WAV data size: {} bytes (in ShmBytes)", wav_size);

        // Open output file via FsService
        let open_result = services.fs.open(self.output_path.clone().into(), FileOpenOptions::create_write()).await
            .wrap_err("RPC call to fs.open failed")?;

        let file_handle = match open_result {
            FileOpenResult::Ok(handle) => handle,
            FileOpenResult::Err(e) => {
                bail!("Failed to open output file: {e}");
            }
        };

        println!("📂 Opened output file: {:?}", self.output_path);

        // Write the ShmBytes to the file (zero-copy from SHM!)
        println!("💾 Writing audio data via FsService (zero-copy from SHM)...");

        let write_result = services.fs.write(file_handle, audio_segment.bytes).await
            .wrap_err("RPC call to fs.write failed")?;

        match write_result {
            FileWriteResult::Ok(bytes_written) => {
                println!("✅ Wrote {} bytes to file", bytes_written);
            }
            FileWriteResult::Err(e) => {
                bail!("Failed to write to file: {e}");
            }
        }

        // Close the file
        let close_result = services.fs.close(file_handle).await
            .wrap_err("RPC call to fs.close failed")?;

        match close_result {
            FileCloseResult::Ok => {
                println!("📁 File closed");
            }
            FileCloseResult::Err(e) => {
                bail!("Failed to close file: {e}");
            }
        }

        println!();
        println!("🎉 Recording complete!");
        println!("   Output: {:?}", self.output_path);
        println!();
        println!("📋 What just happened:");
        println!("   1. MicrophoneService captured audio into shared memory");
        println!("   2. drain_to_wav() created WAV in ShmBytes (no copy)");
        println!("   3. FsService.write() wrote directly from ShmBytes (no copy)");
        println!("   → Audio data never left shared memory until disk write!");

        Ok(())
    }
}

impl ToArgs for MicRecordArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![
            "--id".into(),
            self.id.clone().into(),
            "--duration".into(),
            self.duration.clone().into(),
            "--output-path".into(),
            self.output_path.as_os_str().to_owned(),
        ]
    }
}
