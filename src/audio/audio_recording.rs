//! Audio recording using Windows WASAPI (Windows Audio Session API).
//!
//! This module provides functionality to record audio from a specific microphone
//! device using the low-level WASAPI interface.

use crate::com::com_guard::ComGuard;
use eyre::{Context, Result, bail};
use std::io::Cursor;
use std::ptr;
use std::slice;
use std::time::{Duration, Instant};
use widestring::U16CString;
use windows::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_SHARED, IAudioCaptureClient, IAudioClient,
    IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::PCWSTR;

/// Records audio from a specific device for the given duration.
///
/// Returns the recorded audio as WAV file bytes.
pub fn record_audio(device_id: &str, duration_ms: u64) -> Result<Vec<u8>> {
    let _com_guard = ComGuard::new()?;

    // Get the device by ID
    let device = get_device_by_id(device_id)?;

    // Activate the audio client
    let audio_client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .wrap_err("Failed to activate audio client")?;

    // Get the mix format (the format the device will capture in)
    let mix_format_ptr = unsafe { audio_client.GetMixFormat() }
        .wrap_err("Failed to get mix format")?;

    // SAFETY: GetMixFormat returns a valid pointer that we must free with CoTaskMemFree
    // Copy the fields we need to avoid unaligned reference issues (WAVEFORMATEX is packed)
    let (n_channels, n_samples_per_sec, n_block_align, w_bits_per_sample) = unsafe {
        let fmt = &*mix_format_ptr;
        (fmt.nChannels, fmt.nSamplesPerSec, fmt.nBlockAlign, fmt.wBitsPerSample)
    };

    // Initialize the audio client for capture
    // Using 100-nanosecond units for buffer duration (1 second = 10_000_000)
    let buffer_duration = 10_000_000i64; // 1 second buffer

    unsafe {
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0, // No flags for normal capture (not loopback)
            buffer_duration,
            0, // periodicity (0 = use default)
            mix_format_ptr,
            None, // audio session GUID
        )
    }
    .wrap_err("Failed to initialize audio client")?;

    // Get the capture client interface
    let capture_client: IAudioCaptureClient = unsafe { audio_client.GetService() }
        .wrap_err("Failed to get capture client")?;

    // Get the buffer size
    let buffer_frame_count =
        unsafe { audio_client.GetBufferSize() }.wrap_err("Failed to get buffer size")?;

    tracing::debug!(
        "Audio capture initialized: {} channels, {} Hz, {} bits, buffer frames: {}",
        n_channels,
        n_samples_per_sec,
        w_bits_per_sample,
        buffer_frame_count
    );

    // Prepare to collect audio data
    let bytes_per_frame = n_block_align as usize;
    let mut audio_data: Vec<u8> = Vec::new();

    // Start capturing
    unsafe { audio_client.Start() }.wrap_err("Failed to start audio capture")?;

    let start_time = Instant::now();
    let target_duration = Duration::from_millis(duration_ms);

    // Capture loop
    while start_time.elapsed() < target_duration {
        // Get the next packet size
        let packet_length = unsafe { capture_client.GetNextPacketSize() }
            .wrap_err("Failed to get next packet size")?;

        if packet_length == 0 {
            // No data available, sleep briefly
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        // Get the buffer
        let mut data_ptr: *mut u8 = ptr::null_mut();
        let mut num_frames_available: u32 = 0;
        let mut flags: u32 = 0;

        unsafe {
            capture_client.GetBuffer(
                &mut data_ptr,
                &mut num_frames_available,
                &mut flags,
                None,
                None,
            )
        }
        .wrap_err("Failed to get capture buffer")?;

        if num_frames_available > 0 && !data_ptr.is_null() {
            let data_size = num_frames_available as usize * bytes_per_frame;

            // SAFETY: data_ptr is valid and points to data_size bytes
            let captured_data = unsafe { slice::from_raw_parts(data_ptr, data_size) };

            // Check for silence flag
            const AUDCLNT_BUFFERFLAGS_SILENT: u32 = 0x2;
            if flags & AUDCLNT_BUFFERFLAGS_SILENT != 0 {
                // Device is reporting silence, write zeros
                audio_data.extend(std::iter::repeat(0u8).take(data_size));
            } else {
                audio_data.extend_from_slice(captured_data);
            }
        }

        // Release the buffer
        unsafe { capture_client.ReleaseBuffer(num_frames_available) }
            .wrap_err("Failed to release buffer")?;
    }

    // Stop capturing
    unsafe { audio_client.Stop() }.wrap_err("Failed to stop audio capture")?;

    // Free the mix format
    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
    }

    tracing::info!(
        "Captured {} bytes of audio data ({:.2} seconds)",
        audio_data.len(),
        duration_ms as f64 / 1000.0
    );

    // Convert to WAV format
    let wav_bytes = create_wav_file(&audio_data, n_channels, n_samples_per_sec, w_bits_per_sample)?;

    Ok(wav_bytes)
}

/// Gets an IMMDevice by its device ID string.
fn get_device_by_id(device_id: &str) -> Result<IMMDevice> {
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .wrap_err("Failed to create device enumerator")?;

    // Convert device ID to wide string
    let device_id_wide =
        U16CString::from_str(device_id).wrap_err("Failed to convert device ID to wide string")?;

    let device = unsafe { enumerator.GetDevice(PCWSTR(device_id_wide.as_ptr())) }
        .wrap_err_with(|| format!("Failed to get device with ID: {}", device_id))?;

    Ok(device)
}

/// Creates a WAV file from raw audio data.
fn create_wav_file(
    audio_data: &[u8],
    n_channels: u16,
    n_samples_per_sec: u32,
    w_bits_per_sample: u16,
) -> Result<Vec<u8>> {
    let mut output = Cursor::new(Vec::new());

    let spec = hound::WavSpec {
        channels: n_channels,
        sample_rate: n_samples_per_sec,
        bits_per_sample: w_bits_per_sample,
        sample_format: if w_bits_per_sample == 32 {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        },
    };

    let mut writer =
        hound::WavWriter::new(&mut output, spec).wrap_err("Failed to create WAV writer")?;

    // Write samples based on bit depth
    match w_bits_per_sample {
        16 => {
            // 16-bit samples
            for chunk in audio_data.chunks_exact(2) {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                writer
                    .write_sample(sample)
                    .wrap_err("Failed to write sample")?;
            }
        }
        32 => {
            // 32-bit float samples
            for chunk in audio_data.chunks_exact(4) {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                writer
                    .write_sample(sample)
                    .wrap_err("Failed to write sample")?;
            }
        }
        bits => {
            bail!("Unsupported bit depth: {}", bits);
        }
    }

    writer.finalize().wrap_err("Failed to finalize WAV file")?;

    Ok(output.into_inner())
}
