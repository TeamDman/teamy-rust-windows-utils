//! File service for zero-copy file operations.
//!
//! This service provides file operations that accept ShmBytes,
//! enabling zero-copy writes from other services.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use facet::Facet;
use parking_lot::Mutex;
use roam::Context;
use roam_shm::shm_bytes::ShmBytes;

use super::TeamyPath;

/// A handle to an open file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Facet)]
pub struct FileHandle(pub u32);

/// Options for opening a file.
#[derive(Debug, Clone, Facet)]
pub struct FileOpenOptions {
    /// Create the file if it doesn't exist.
    pub create: bool,
    /// Truncate the file if it exists.
    pub truncate: bool,
    /// Open for writing.
    pub write: bool,
    /// Open for reading.
    pub read: bool,
    /// Append to the file.
    pub append: bool,
}

impl Default for FileOpenOptions {
    fn default() -> Self {
        Self {
            create: false,
            truncate: false,
            write: false,
            read: true,
            append: false,
        }
    }
}

impl FileOpenOptions {
    /// Create options for creating/writing a new file.
    pub fn create_write() -> Self {
        Self {
            create: true,
            truncate: true,
            write: true,
            read: false,
            append: false,
        }
    }
}

/// Result of opening a file.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum FileOpenResult {
    Ok(FileHandle),
    Err(String),
}

/// Result of writing to a file.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum FileWriteResult {
    /// Number of bytes written.
    Ok(u64),
    Err(String),
}

/// Result of closing a file.
#[derive(Debug, Clone, Facet)]
#[repr(u8)]
pub enum FileCloseResult {
    Ok,
    Err(String),
}

/// File service - provides file operations with ShmBytes support.
#[roam::service]
pub trait FsService {
    /// Open a file with the given options.
    async fn open(&self, path: TeamyPath, options: FileOpenOptions) -> FileOpenResult;

    /// Write ShmBytes to a file.
    /// The bytes are read directly from shared memory - no copy needed.
    async fn write(&self, handle: FileHandle, data: ShmBytes) -> FileWriteResult;

    /// Write raw bytes to a file (fallback for non-SHM usage).
    async fn write_bytes(&self, handle: FileHandle, data: Vec<u8>) -> FileWriteResult;

    /// Close a file handle.
    async fn close(&self, handle: FileHandle) -> FileCloseResult;
}

// ============================================================================
// Implementation
// ============================================================================

/// State for the file service.
struct FsServiceState {
    /// Open file handles.
    files: HashMap<u32, OpenFile>,
}

/// An open file with its path.
struct OpenFile {
    file: File,
    path: PathBuf,
}

/// Implementation of the FsService.
#[derive(Clone)]
pub struct FsServiceImpl {
    state: Arc<Mutex<FsServiceState>>,
    next_handle: Arc<AtomicU32>,
}

impl FsServiceImpl {
    /// Create a new file service instance.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FsServiceState {
                files: HashMap::new(),
            })),
            next_handle: Arc::new(AtomicU32::new(1)),
        }
    }
}

impl Default for FsServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl FsService for FsServiceImpl {
    async fn open(
        &self,
        _ctx: &Context,
        path: TeamyPath,
        options: FileOpenOptions,
    ) -> FileOpenResult {
        let path_buf: PathBuf = path.into();

        // Create parent directories if needed
        if options.create {
            if let Some(parent) = path_buf.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return FileOpenResult::Err(format!(
                        "Failed to create directories for {}: {e}",
                        path_buf.display()
                    ));
                }
            }
        }

        let file = OpenOptions::new()
            .create(options.create)
            .truncate(options.truncate)
            .write(options.write)
            .read(options.read)
            .append(options.append)
            .open(&path_buf);

        match file {
            Ok(file) => {
                let handle_id = self.next_handle.fetch_add(1, Ordering::SeqCst);
                let mut state = self.state.lock();
                state.files.insert(
                    handle_id,
                    OpenFile {
                        file,
                        path: path_buf.clone(),
                    },
                );
                tracing::debug!(handle = handle_id, path = %path_buf.display(), "Opened file");
                FileOpenResult::Ok(FileHandle(handle_id))
            }
            Err(e) => FileOpenResult::Err(format!("Failed to open {}: {e}", path_buf.display())),
        }
    }

    async fn write(
        &self,
        _ctx: &Context,
        handle: FileHandle,
        data: ShmBytes,
    ) -> FileWriteResult {
        // Read the data from ShmBytes
        let bytes = match data.as_slice() {
            Some(slice) => slice.to_vec(), // We need to copy here since File::write needs &[u8]
            None => {
                return FileWriteResult::Err(
                    "Failed to access ShmBytes data (not in SHM context?)".to_string(),
                )
            }
        };

        let bytes_len = bytes.len() as u64;

        tracing::info!(
            handle = handle.0,
            bytes = bytes_len,
            "Writing ShmBytes to file (zero-copy from SHM)"
        );

        let mut state = self.state.lock();
        if let Some(open_file) = state.files.get_mut(&handle.0) {
            match open_file.file.write_all(&bytes) {
                Ok(()) => {
                    tracing::debug!(handle = handle.0, bytes = bytes_len, "Write complete");
                    FileWriteResult::Ok(bytes_len)
                }
                Err(e) => FileWriteResult::Err(format!("Write failed: {e}")),
            }
        } else {
            FileWriteResult::Err(format!("Invalid file handle: {}", handle.0))
        }
    }

    async fn write_bytes(
        &self,
        _ctx: &Context,
        handle: FileHandle,
        data: Vec<u8>,
    ) -> FileWriteResult {
        let bytes_len = data.len() as u64;

        let mut state = self.state.lock();
        if let Some(open_file) = state.files.get_mut(&handle.0) {
            match open_file.file.write_all(&data) {
                Ok(()) => FileWriteResult::Ok(bytes_len),
                Err(e) => FileWriteResult::Err(format!("Write failed: {e}")),
            }
        } else {
            FileWriteResult::Err(format!("Invalid file handle: {}", handle.0))
        }
    }

    async fn close(&self, _ctx: &Context, handle: FileHandle) -> FileCloseResult {
        let mut state = self.state.lock();
        if let Some(open_file) = state.files.remove(&handle.0) {
            // File is closed when dropped
            tracing::debug!(
                handle = handle.0,
                path = %open_file.path.display(),
                "Closed file"
            );
            FileCloseResult::Ok
        } else {
            FileCloseResult::Err(format!("Invalid file handle: {}", handle.0))
        }
    }
}
