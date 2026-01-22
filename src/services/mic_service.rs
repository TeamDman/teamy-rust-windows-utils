//! Microphone service for audio recording.
//!
//! This service manages microphone recording sessions and can produce
//! audio data as ShmBytes for zero-copy transfer.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use eyre::{Context as _, bail};
use facet::Facet;
use jiff::Timestamp;
use parking_lot::Mutex;
use roam::Context;
use roam_shm::shm_bytes::ShmBytes;

/// Information about a microphone device.
#[derive(Debug, Clone, Facet)]
pub struct MicrophoneInfo {
    /// The unique device ID (Windows IMM device ID).
    pub id: String,
    /// The friendly name of the microphone.
    pub name: String,
    /// Whether this is the default microphone.
    pub is_default: bool,
}

/// Result of listing microphones.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum ListMicrophonesResult {
    Ok(Vec<MicrophoneInfo>),
    Err(String),
}

/// Result of starting a recording.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum StartRecordingResult {
    Ok,
    Err(String),
}

/// Result of stopping a recording.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum StopRecordingResult {
    Ok,
    Err(String),
}

/// Audio data with format information, wrapped around ShmBytes.
#[derive(Facet)]
pub struct AudioSegment {
    /// The audio data as a WAV file in shared memory.
    pub bytes: ShmBytes,
    /// Duration of the audio in milliseconds.
    pub duration_ms: u64,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
    /// Bits per sample.
    pub bits_per_sample: u16,
}

/// Result of draining recorded audio.
#[derive(Facet)]
#[repr(u8)]
pub enum DrainAudioResult {
    Ok(AudioSegment),
    Err(String),
}

/// Microphone service - manages audio recording sessions.
#[roam::service]
pub trait MicrophoneService {
    /// List all available microphone devices.
    async fn list(&self) -> ListMicrophonesResult;

    /// Start recording from a microphone.
    async fn start_recording(&self, device_id: String) -> StartRecordingResult;

    /// Stop recording from a microphone.
    async fn stop_recording(&self, device_id: String) -> StopRecordingResult;

    /// Drain recorded audio as a WAV file in ShmBytes.
    /// This consumes all recorded data for the device.
    async fn drain_to_wav(&self, device_id: String) -> DrainAudioResult;
}

// ============================================================================
// Recording State
// ============================================================================

/// State for an active recording session.
struct RecordingSession {
    /// When the recording started.
    started_at: Timestamp,
    /// Audio format info.
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    bytes_per_frame: usize,
    /// Accumulated raw audio data.
    audio_data: Vec<u8>,
    /// Whether recording is currently active.
    is_recording: bool,
    /// Handle to the recording thread (join handle).
    recording_thread: Option<std::thread::JoinHandle<()>>,
    /// Channel to signal stop.
    stop_tx: Option<std::sync::mpsc::Sender<()>>,
}

/// Shared state for the microphone service.
struct MicServiceState {
    /// Active recording sessions by device ID.
    sessions: HashMap<String, RecordingSession>,
}

/// Implementation of the MicrophoneService.
#[derive(Clone)]
pub struct MicrophoneServiceImpl {
    state: Arc<Mutex<MicServiceState>>,
}

impl MicrophoneServiceImpl {
    /// Create a new microphone service instance.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MicServiceState {
                sessions: HashMap::new(),
            })),
        }
    }
}

impl Default for MicrophoneServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl MicrophoneService for MicrophoneServiceImpl {
    async fn list(&self, _ctx: &Context) -> ListMicrophonesResult {
        match crate::audio::list_audio_input_devices() {
            Ok(devices) => {
                let mics = devices
                    .into_iter()
                    .map(|d| MicrophoneInfo {
                        id: d.id.0,
                        name: d.name,
                        is_default: d.is_default,
                    })
                    .collect();
                ListMicrophonesResult::Ok(mics)
            }
            Err(e) => ListMicrophonesResult::Err(format!("{e:#}")),
        }
    }

    async fn start_recording(&self, _ctx: &Context, device_id: String) -> StartRecordingResult {
        // Check if already recording
        {
            let state = self.state.lock();
            if state.sessions.contains_key(&device_id) {
                return StartRecordingResult::Err(format!(
                    "Already recording from device {device_id}"
                ));
            }
        }

        // Create channel to signal stop
        let (stop_tx, stop_rx) = std::sync::mpsc::channel::<()>();
        // Channel to report errors during startup
        let (error_tx, error_rx) = std::sync::mpsc::channel::<String>();

        let device_id_clone = device_id.clone();
        let state_clone = self.state.clone();

        // Spawn recording thread
        let handle = std::thread::spawn(move || {
            match run_recording_thread(&device_id_clone, stop_rx) {
                Ok(result) => {
                    // Store the audio data in the session
                    let mut state = state_clone.lock();
                    if let Some(session) = state.sessions.get_mut(&device_id_clone) {
                        session.audio_data = result.audio_data;
                        session.sample_rate = result.sample_rate;
                        session.channels = result.channels;
                        session.bits_per_sample = result.bits_per_sample;
                        session.bytes_per_frame = result.bytes_per_frame;
                        session.is_recording = false;
                    }
                }
                Err(e) => {
                    // Report error and clean up session
                    let _ = error_tx.send(e);
                    let mut state = state_clone.lock();
                    state.sessions.remove(&device_id_clone);
                }
            }
        });

        // Initialize the session
        {
            let mut state = self.state.lock();
            state.sessions.insert(
                device_id.clone(),
                RecordingSession {
                    started_at: Timestamp::now(),
                    sample_rate: 0,
                    channels: 0,
                    bits_per_sample: 0,
                    bytes_per_frame: 0,
                    audio_data: Vec::new(),
                    is_recording: true,
                    recording_thread: Some(handle),
                    stop_tx: Some(stop_tx),
                },
            );
        }

        // Wait briefly for any immediate startup errors
        match error_rx.recv_timeout(Duration::from_millis(500)) {
            Ok(e) => {
                // Thread encountered an error during startup
                let mut state = self.state.lock();
                state.sessions.remove(&device_id);
                StartRecordingResult::Err(e)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Thread is running - this is the expected case
                tracing::info!(device_id, "Recording started");
                StartRecordingResult::Ok
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Thread finished without error (dropped the sender)
                // This could happen if recording finished very quickly
                StartRecordingResult::Ok
            }
        }
    }

    async fn stop_recording(&self, _ctx: &Context, device_id: String) -> StopRecordingResult {
        let (stop_tx, thread_handle) = {
            let mut state = self.state.lock();
            if let Some(session) = state.sessions.get_mut(&device_id) {
                if !session.is_recording {
                    return StopRecordingResult::Err("Recording already stopped".to_string());
                }
                (session.stop_tx.take(), session.recording_thread.take())
            } else {
                return StopRecordingResult::Err(format!("No recording session for device {device_id}"));
            }
        };

        // Signal the thread to stop
        if let Some(tx) = stop_tx {
            let _ = tx.send(());
        }

        // Wait for the thread to finish
        if let Some(handle) = thread_handle {
            match handle.join() {
                Ok(()) => {
                    tracing::info!(device_id, "Recording stopped");
                    StopRecordingResult::Ok
                }
                Err(_) => {
                    StopRecordingResult::Err("Recording thread panicked".to_string())
                }
            }
        } else {
            StopRecordingResult::Ok
        }
    }

    async fn drain_to_wav(&self, _ctx: &Context, device_id: String) -> DrainAudioResult {
        let session_data = {
            let mut state = self.state.lock();

            if let Some(session) = state.sessions.get(&device_id) {
                if session.is_recording {
                    return DrainAudioResult::Err("Recording still in progress - call stop_recording first".to_string());
                }
            }

            state.sessions.remove(&device_id)
        };

        let Some(session) = session_data else {
            return DrainAudioResult::Err(format!("No recording data for device {device_id}"));
        };

        let (audio_data, sample_rate, channels, bits_per_sample) = (
            session.audio_data,
            session.sample_rate,
            session.channels,
            session.bits_per_sample,
        );

        if audio_data.is_empty() {
            return DrainAudioResult::Err("No audio data recorded".to_string());
        }

        // Convert to WAV
        let wav_bytes = match create_wav_file(&audio_data, channels, sample_rate, bits_per_sample) {
            Ok(bytes) => bytes,
            Err(e) => return DrainAudioResult::Err(format!("Failed to create WAV: {e:#}")),
        };

        // Calculate duration
        let bytes_per_sample = bits_per_sample as usize / 8;
        let bytes_per_frame = bytes_per_sample * channels as usize;
        let total_frames = if bytes_per_frame > 0 {
            audio_data.len() / bytes_per_frame
        } else {
            0
        };
        let duration_ms = if sample_rate > 0 {
            (total_frames as u64 * 1000) / sample_rate as u64
        } else {
            0
        };

        // Allocate ShmBytes and copy the WAV data
        let shm_bytes = match ShmBytes::alloc(wav_bytes.len()) {
            Ok(mut bytes) => {
                if let Some(slice) = bytes.as_mut_slice() {
                    slice.copy_from_slice(&wav_bytes);
                }
                bytes
            }
            Err(e) => return DrainAudioResult::Err(format!("Failed to allocate ShmBytes: {e}")),
        };

        tracing::info!(
            device_id,
            wav_size = wav_bytes.len(),
            duration_ms,
            "Drained audio to WAV"
        );

        DrainAudioResult::Ok(AudioSegment {
            bytes: shm_bytes,
            duration_ms,
            sample_rate,
            channels,
            bits_per_sample,
        })
    }
}

// ============================================================================
// Recording Thread
// ============================================================================

struct RecordingThreadResult {
    audio_data: Vec<u8>,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    bytes_per_frame: usize,
}

fn run_recording_thread(
    device_id: &str,
    stop_rx: std::sync::mpsc::Receiver<()>,
) -> Result<RecordingThreadResult, String> {
    use crate::com::com_guard::ComGuard;
    use std::ptr;
    use std::slice;
    use widestring::U16CString;
    use windows::Win32::Media::Audio::{
        AUDCLNT_SHAREMODE_SHARED, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
    use windows::core::PCWSTR;

    let _com_guard = ComGuard::new().map_err(|e| format!("COM init failed: {e}"))?;

    // Get the device
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|e| format!("Failed to create device enumerator: {e}"))?;

    let device_id_wide = U16CString::from_str(device_id)
        .map_err(|e| format!("Failed to convert device ID: {e}"))?;

    let device = unsafe { enumerator.GetDevice(PCWSTR(device_id_wide.as_ptr())) }
        .map_err(|e| format!("Failed to get device {device_id}: {e}"))?;

    // Activate audio client
    let audio_client: IAudioClient = unsafe { device.Activate(CLSCTX_ALL, None) }
        .map_err(|e| format!("Failed to activate audio client: {e}"))?;

    // Get mix format
    let mix_format_ptr = unsafe { audio_client.GetMixFormat() }
        .map_err(|e| format!("Failed to get mix format: {e}"))?;

    let (n_channels, n_samples_per_sec, n_block_align, w_bits_per_sample) = unsafe {
        let fmt = &*mix_format_ptr;
        (fmt.nChannels, fmt.nSamplesPerSec, fmt.nBlockAlign, fmt.wBitsPerSample)
    };

    // Initialize audio client
    let buffer_duration = 10_000_000i64; // 1 second
    unsafe {
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            buffer_duration,
            0,
            mix_format_ptr,
            None,
        )
    }
    .map_err(|e| format!("Failed to initialize audio client: {e}"))?;

    let capture_client: IAudioCaptureClient = unsafe { audio_client.GetService() }
        .map_err(|e| format!("Failed to get capture client: {e}"))?;

    // Start capturing
    unsafe { audio_client.Start() }
        .map_err(|e| format!("Failed to start capture: {e}"))?;

    tracing::debug!(device_id, "Recording thread started");

    let bytes_per_frame = n_block_align as usize;
    let mut audio_data = Vec::new();

    // Capture loop
    loop {
        // Check for stop signal (non-blocking)
        match stop_rx.try_recv() {
            Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                tracing::debug!(device_id, "Stop signal received");
                break;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // Continue recording
            }
        }

        let packet_length = match unsafe { capture_client.GetNextPacketSize() } {
            Ok(len) => len,
            Err(e) => {
                tracing::warn!(device_id, error = %e, "GetNextPacketSize failed");
                break;
            }
        };

        if packet_length == 0 {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        let mut data_ptr: *mut u8 = ptr::null_mut();
        let mut num_frames: u32 = 0;
        let mut flags: u32 = 0;

        if let Err(e) = unsafe {
            capture_client.GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)
        } {
            tracing::warn!(device_id, error = %e, "GetBuffer failed");
            break;
        }

        if num_frames > 0 && !data_ptr.is_null() {
            let data_size = num_frames as usize * bytes_per_frame;
            let captured = unsafe { slice::from_raw_parts(data_ptr, data_size) };

            const AUDCLNT_BUFFERFLAGS_SILENT: u32 = 0x2;
            if flags & AUDCLNT_BUFFERFLAGS_SILENT != 0 {
                audio_data.extend(std::iter::repeat(0u8).take(data_size));
            } else {
                audio_data.extend_from_slice(captured);
            }
        }

        if let Err(e) = unsafe { capture_client.ReleaseBuffer(num_frames) } {
            tracing::warn!(device_id, error = %e, "ReleaseBuffer failed");
            break;
        }
    }

    // Stop and cleanup
    let _ = unsafe { audio_client.Stop() };
    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
    }

    tracing::debug!(
        device_id,
        bytes = audio_data.len(),
        "Recording thread finished"
    );

    Ok(RecordingThreadResult {
        audio_data,
        sample_rate: n_samples_per_sec,
        channels: n_channels,
        bits_per_sample: w_bits_per_sample,
        bytes_per_frame,
    })
}

/// Creates a WAV file from raw audio data.
fn create_wav_file(
    audio_data: &[u8],
    n_channels: u16,
    n_samples_per_sec: u32,
    w_bits_per_sample: u16,
) -> eyre::Result<Vec<u8>> {
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

    let mut writer = hound::WavWriter::new(&mut output, spec)
        .wrap_err("Failed to create WAV writer")?;

    match w_bits_per_sample {
        16 => {
            for chunk in audio_data.chunks_exact(2) {
                let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                writer.write_sample(sample).wrap_err("Failed to write sample")?;
            }
        }
        32 => {
            for chunk in audio_data.chunks_exact(4) {
                let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                writer.write_sample(sample).wrap_err("Failed to write sample")?;
            }
        }
        bits => bail!("Unsupported bit depth: {bits}"),
    }

    writer.finalize().wrap_err("Failed to finalize WAV")?;
    Ok(output.into_inner())
}
