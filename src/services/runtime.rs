//! Roam SHM runtime for teamy-windows services.
//!
//! This module provides a runtime that sets up roam-shm infrastructure
//! for zero-copy ShmBytes transfers between services.

use std::sync::Arc;

use roam_shm::driver::{establish_guest, establish_multi_peer_host, ShmConnectionHandle};
use roam_shm::host::ShmHost;
use roam_shm::layout::{SegmentConfig, SizeClass};
use roam_shm::shm_bytes::{SHM_LOCAL_PEER_ID, SHM_POOL};
use roam_shm::spawn::AddPeerOptions;
use roam_shm::transport::ShmGuestTransport;
use roam_shm::var_slot_pool::VarSlotPool;

use super::fs_service::{FsServiceClient, FsServiceDispatcher, FsServiceImpl};
use super::mic_service::{MicrophoneServiceClient, MicrophoneServiceDispatcher, MicrophoneServiceImpl};

/// The teamy-windows service runtime.
///
/// Sets up roam-shm transport with both MicrophoneService and FsService.
/// Provides clients for calling these services with ShmBytes support.
pub struct ServiceRuntime {
    /// Client for calling MicrophoneService.
    pub mic: MicrophoneServiceClient<ShmConnectionHandle>,
    /// Client for calling FsService.
    pub fs: FsServiceClient<ShmConnectionHandle>,
    /// The SHM pool for manual ShmBytes access.
    pub pool: Arc<VarSlotPool>,
    /// Temp directory for SHM segment (kept alive).
    _shm_dir: tempfile::TempDir,
}

impl ServiceRuntime {
    /// Create a new service runtime with roam-shm transport.
    ///
    /// This sets up:
    /// - A shared memory segment with variable-size slots for ShmBytes
    /// - MicrophoneService (host-side, handles recording)
    /// - FsService (host-side, handles file I/O)
    /// - Clients for calling both services
    ///
    /// The drivers are spawned as tokio tasks.
    pub async fn new() -> eyre::Result<Self> {
        let shm_dir = tempfile::tempdir()?;
        let shm_path = shm_dir.path().join("teamy-windows.shm");

        tracing::debug!(path = %shm_path.display(), "Creating SHM segment");

        // Configure SHM segment with variable-size slot classes for audio data
        let config = SegmentConfig {
            max_payload_size: 64 * 1024, // 64KB max message payload
            var_slot_classes: Some(vec![
                // Small buffers (metadata, small messages)
                SizeClass::new(256, 32),
                // Medium buffers (short audio clips)
                SizeClass::new(4 * 1024, 16),
                // Large buffers (audio segments ~1 second at 48kHz stereo 16-bit = 192KB)
                SizeClass::new(64 * 1024, 8),
                // Very large buffers (longer recordings)
                SizeClass::new(256 * 1024, 4),
                // Huge buffers (full recordings up to ~30 seconds)
                SizeClass::new(1024 * 1024, 4),
                // Extra large for longer recordings
                SizeClass::new(4 * 1024 * 1024, 2),
            ]),
            ..SegmentConfig::default()
        };

        let mut host = ShmHost::create(&shm_path, config)?;
        let pool = host
            .var_slot_pool()
            .expect("SHM host should have var_slot_pool");

        // Add a peer for the "guest" (our CLI/caller side)
        let ticket = host.add_peer(AddPeerOptions {
            peer_name: Some("teamy-cli".to_string()),
            ..Default::default()
        })?;

        let peer_id = ticket.peer_id;
        let spawn_args = ticket.into_spawn_args();

        // === Host side: Services ===
        // We'll create a combined dispatcher that handles both services.
        // For simplicity, we run MicrophoneService on host.
        let mic_impl = MicrophoneServiceImpl::new();
        let mic_dispatcher = MicrophoneServiceDispatcher::new(mic_impl);

        // === Guest side: FsService ===
        // FsService runs on the guest side so it can write files.
        let fs_impl = FsServiceImpl::new();
        let fs_dispatcher = FsServiceDispatcher::new(fs_impl);

        // Set up guest transport
        let guest_transport = ShmGuestTransport::from_spawn_args(spawn_args)?;
        let (guest_handle, guest_driver) = establish_guest(guest_transport, fs_dispatcher);

        // Set up host driver
        let (host_driver, mut handles, _) =
            establish_multi_peer_host(host, vec![(peer_id, mic_dispatcher)]);
        let host_handle = handles.remove(&peer_id).expect("should have peer handle");

        // Spawn the drivers
        tokio::spawn(async move {
            if let Err(e) = guest_driver.run().await {
                tracing::error!("Guest driver error: {e:?}");
            }
        });
        tokio::spawn(async move {
            if let Err(e) = host_driver.run().await {
                tracing::error!("Host driver error: {e:?}");
            }
        });

        // Create clients
        // - guest_handle calls INTO the host (where MicrophoneService is)
        // - host_handle calls INTO the guest (where FsService is)
        let mic_client = MicrophoneServiceClient::new(guest_handle);
        let fs_client = FsServiceClient::new(host_handle);

        tracing::info!("Service runtime initialized with roam-shm transport");

        Ok(Self {
            mic: mic_client,
            fs: fs_client,
            pool,
            _shm_dir: shm_dir,
        })
    }

    /// Run a closure within the SHM context.
    ///
    /// This is needed to access ShmBytes data outside of service dispatch.
    pub fn with_shm_context<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        SHM_POOL.sync_scope(self.pool.clone(), || {
            SHM_LOCAL_PEER_ID.sync_scope(0, f)
        })
    }

    /// Run an async closure within the SHM context.
    pub async fn with_shm_context_async<F, Fut, R>(&self, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let pool = self.pool.clone();
        SHM_POOL
            .scope(pool, async {
                SHM_LOCAL_PEER_ID.scope(0, f()).await
            })
            .await
    }
}
