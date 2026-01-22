//! Microphone service for audio recording.
//!
//! This service manages microphone recording sessions and can produce
//! audio data as ShmBytes for zero-copy transfer.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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

        // Calculate duration before we move audio_data
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

        // Determine header size based on format
        // >2 channels or >16 bits requires WAVEFORMATEXTENSIBLE (68 byte header)
        // Otherwise use PCMWAVEFORMAT (44 byte header)
        let header_size = if channels > 2 || bits_per_sample > 16 { 68 } else { 44 };
        let wav_size = header_size + audio_data.len();

        // Allocate ShmBytes and write WAV directly into it (no intermediate buffer!)
        let shm_bytes = match ShmBytes::alloc(wav_size) {
            Ok(mut bytes) => {
                match bytes.as_mut_slice() {
                    Some(slice) => {
                        // Write WAV header directly
                        write_wav_header(
                            &mut slice[..header_size],
                            channels,
                            sample_rate,
                            bits_per_sample,
                            audio_data.len() as u32,
                        );
                        // Copy raw audio data directly after header
                        slice[header_size..].copy_from_slice(&audio_data);
                    }
                    None => {
                        return DrainAudioResult::Err("Failed to access ShmBytes slice".to_string());
                    }
                }
                bytes
            }
            Err(e) => return DrainAudioResult::Err(format!("Failed to allocate ShmBytes: {e}")),
        };

        tracing::info!(
            device_id,
            wav_size,
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

/// Writes a WAV header directly into a buffer.
/// 
/// For PCM data, the raw samples can be appended directly after the header.
/// This avoids hound's overhead and allows writing directly to ShmBytes.
fn write_wav_header(
    buf: &mut [u8],
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    data_size: u32,
) {
    let bytes_per_sample = (bits_per_sample + 7) / 8;
    let block_align = bytes_per_sample as u16 * channels;
    let byte_rate = sample_rate * block_align as u32;
    
    // Use WAVEFORMATEXTENSIBLE for >2 channels or >16 bits
    let use_extensible = channels > 2 || bits_per_sample > 16;
    
    if use_extensible {
        write_wav_header_extensible(buf, channels, sample_rate, bits_per_sample, bytes_per_sample, block_align, byte_rate, data_size);
    } else {
        write_wav_header_pcm(buf, channels, sample_rate, bits_per_sample, block_align, byte_rate, data_size);
    }
}

/// Write PCMWAVEFORMAT header (44 bytes)
fn write_wav_header_pcm(
    buf: &mut [u8],
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    block_align: u16,
    byte_rate: u32,
    data_size: u32,
) {
    let file_size = 36 + data_size; // Total - 8 bytes for RIFF header
    
    // RIFF header
    buf[0..4].copy_from_slice(b"RIFF");
    buf[4..8].copy_from_slice(&file_size.to_le_bytes());
    buf[8..12].copy_from_slice(b"WAVE");
    
    // fmt chunk
    buf[12..16].copy_from_slice(b"fmt ");
    buf[16..20].copy_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    
    // Format tag: 1 = PCM, 3 = IEEE float
    let format_tag: u16 = if bits_per_sample == 32 { 3 } else { 1 };
    buf[20..22].copy_from_slice(&format_tag.to_le_bytes());
    buf[22..24].copy_from_slice(&channels.to_le_bytes());
    buf[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    buf[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    buf[32..34].copy_from_slice(&block_align.to_le_bytes());
    buf[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
    
    // data chunk
    buf[36..40].copy_from_slice(b"data");
    buf[40..44].copy_from_slice(&data_size.to_le_bytes());
}

/// Write WAVEFORMATEXTENSIBLE header (68 bytes)
fn write_wav_header_extensible(
    buf: &mut [u8],
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    bytes_per_sample: u16,
    block_align: u16,
    byte_rate: u32,
    data_size: u32,
) {
    let file_size = 60 + data_size; // Total - 8 bytes for RIFF header
    
    // RIFF header
    buf[0..4].copy_from_slice(b"RIFF");
    buf[4..8].copy_from_slice(&file_size.to_le_bytes());
    buf[8..12].copy_from_slice(b"WAVE");
    
    // fmt chunk
    buf[12..16].copy_from_slice(b"fmt ");
    buf[16..20].copy_from_slice(&40u32.to_le_bytes()); // fmt chunk size for extensible
    buf[20..22].copy_from_slice(&0xFFFEu16.to_le_bytes()); // WAVE_FORMAT_EXTENSIBLE
    buf[22..24].copy_from_slice(&channels.to_le_bytes());
    buf[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    buf[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    buf[32..34].copy_from_slice(&block_align.to_le_bytes());
    buf[34..36].copy_from_slice(&(bytes_per_sample * 8).to_le_bytes()); // Container bits
    buf[36..38].copy_from_slice(&22u16.to_le_bytes()); // cbSize
    buf[38..40].copy_from_slice(&bits_per_sample.to_le_bytes()); // Valid bits
    
    // Channel mask (default assignment)
    let channel_mask: u32 = match channels {
        1 => 0x4,      // FRONT_CENTER
        2 => 0x3,      // FRONT_LEFT | FRONT_RIGHT
        3 => 0x7,      // + FRONT_CENTER
        4 => 0x33,     // + BACK_LEFT | BACK_RIGHT
        5 => 0x37,     // + FRONT_CENTER
        6 => 0x3F,     // + LOW_FREQUENCY
        7 => 0x13F,    // + BACK_CENTER
        8 => 0x63F,    // + SIDE_LEFT | SIDE_RIGHT
        _ => 0,
    };
    buf[40..44].copy_from_slice(&channel_mask.to_le_bytes());
    
    // SubFormat GUID
    let subformat = if bits_per_sample == 32 {
        // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
        [0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
         0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71]
    } else {
        // KSDATAFORMAT_SUBTYPE_PCM
        [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
         0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71]
    };
    buf[44..60].copy_from_slice(&subformat);
    
    // data chunk
    buf[60..64].copy_from_slice(b"data");
    buf[64..68].copy_from_slice(&data_size.to_le_bytes());
}
