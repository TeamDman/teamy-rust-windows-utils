//! Roam service for microphone management.
//!
//! This module provides a roam service that exposes microphone functionality
//! including listing available microphones and recording audio.

use facet::Facet;
use roam::Context;

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

/// Request to record audio from a microphone.
#[derive(Debug, Clone, Facet)]
pub struct RecordRequest {
    /// The device ID to record from.
    pub device_id: String,
    /// Duration to record in milliseconds.
    pub duration_ms: u64,
}

/// Result of a recording operation.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum RecordResult {
    /// Recording succeeded, contains WAV file bytes.
    Ok(Vec<u8>),
    /// Recording failed with an error message.
    Err(String),
}

/// Result of listing microphones.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum ListMicrophonesResult {
    /// List succeeded.
    Ok(Vec<MicrophoneInfo>),
    /// List failed with an error message.
    Err(String),
}

/// Microphone service - provides access to audio input devices.
///
/// This service can list available microphones and record audio from them.
#[roam::service]
pub trait MicrophoneService {
    /// List all available microphone devices.
    async fn list(&self) -> ListMicrophonesResult;

    /// Record audio from a specific microphone.
    ///
    /// Returns WAV file bytes on success.
    async fn record(&self, request: RecordRequest) -> RecordResult;
}

/// Implementation of the MicrophoneService.
#[derive(Clone)]
pub struct MicrophoneServiceImpl;

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

    async fn record(&self, _ctx: &Context, request: RecordRequest) -> RecordResult {
        // Recording is blocking, so we spawn it on a blocking thread
        let result = tokio::task::spawn_blocking(move || {
            crate::audio::record_audio(&request.device_id, request.duration_ms)
        })
        .await;

        match result {
            Ok(Ok(wav_bytes)) => RecordResult::Ok(wav_bytes),
            Ok(Err(e)) => RecordResult::Err(format!("{e:#}")),
            Err(e) => RecordResult::Err(format!("Task join error: {e:#}")),
        }
    }
}
