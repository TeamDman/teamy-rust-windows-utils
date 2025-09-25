use crate::string::EasyPCWSTR;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;
use eyre::Context;
use uom::si::information::mebibyte;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use windows::Win32::Storage::FileSystem::CreateFileW;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::Storage::FileSystem::FILE_BEGIN;
use windows::Win32::Storage::FileSystem::FILE_END;
use windows::Win32::Storage::FileSystem::FILE_GENERIC_READ;
use windows::Win32::Storage::FileSystem::FILE_SHARE_DELETE;
use windows::Win32::Storage::FileSystem::FILE_SHARE_READ;
use windows::Win32::Storage::FileSystem::FILE_SHARE_WRITE;
use windows::Win32::Storage::FileSystem::OPEN_EXISTING;
use windows::Win32::Storage::FileSystem::ReadFile;
use windows::Win32::Storage::FileSystem::SetFilePointerEx;
use windows::core::Owned;
use uom::si::information::byte;
use uom::si::usize::Information;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WatchInitBehaviour {
    ReadFromStart,
    ReadFromEnd,
}

pub struct WatchConfig {
    pub path: PathBuf,
    pub init_behaviour: WatchInitBehaviour,
    pub read_chunk_size: Information,
}
impl WatchConfig {
    pub fn new_from_start(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            init_behaviour: WatchInitBehaviour::ReadFromStart,
            read_chunk_size: Information::new::<mebibyte>(64),
        }
    }
}

/// Watch a file for appended content. Returns a channel receiver of newly appended byte chunks (may be variable sized).
/// Loop ends when the background thread finishes (currently never unless error). On error, channel is closed.
pub fn watch_file_content(config: WatchConfig) -> eyre::Result<Receiver<Vec<u8>>> {
    let path = config.path;
    if !path.is_file() {
        eyre::bail!("Path is not a file: {}", path.display());
    }
    let path = path.to_path_buf();
    let (tx, rx) = unbounded::<Vec<u8>>();

    // Spawn background reader thread
    thread::Builder::new()
        .name("win-file-content-watch".into())
        .spawn(move || {
            // Open via Win32 CreateFileW with shared access
            let handle = unsafe {
                Owned::new(
                    CreateFileW(
                        path.as_path().easy_pcwstr()?.as_ref(),
                        FILE_GENERIC_READ.0,
                        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                        None,
                        OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL,
                        None,
                    )
                    .with_context(|| {
                        format!("Failed to open file for watching: {}", path.display())
                    })?,
                )
            };

            // Determine starting position
            let mut current_pos: i64 = 0;
            match config.init_behaviour {
                WatchInitBehaviour::ReadFromStart => {
                    unsafe { SetFilePointerEx(*handle, 0, Some(&mut current_pos), FILE_BEGIN) }?
                }
                WatchInitBehaviour::ReadFromEnd => {
                    unsafe { SetFilePointerEx(*handle, 0, Some(&mut current_pos), FILE_END) }?
                }
            }

            let mut buf = vec![0u8; config.read_chunk_size.get::<byte>()];
            loop {
                // Attempt read
                let mut bytes_read: u32 = 0;
                unsafe {
                    ReadFile(
                        *handle,
                        Some(buf.as_mut_slice()),
                        Some(&mut bytes_read),
                        None,
                    )
                    .wrap_err_with(|| format!("ReadFile error watching {}", path.display()))?
                }
                if bytes_read > 0 {
                    current_pos += bytes_read as i64;
                    let chunk = buf[..bytes_read as usize].to_vec();
                    if tx.send(chunk).is_err() {
                        break;
                    }
                    continue; // attempt immediate next read (burst)
                } else {
                    // // No data; check for truncation
                    // use windows::Win32::Storage::FileSystem::GetFileSizeEx;
                    // let mut size: i64 = 0;
                    // if let Err(e) = unsafe { GetFileSizeEx(*handle, &mut size) } {
                    //     eprintln!("GetFileSizeEx error: {e:?}");
                    //     break;
                    // }
                    // if size < current_pos {
                    //     // truncated
                    //     if let Err(e) = unsafe {
                    //         SetFilePointerEx(*handle, 0, Some(&mut current_pos), FILE_BEGIN)
                    //     } {
                    //         eprintln!("Seek reset error: {e:?}");
                    //         break;
                    //     }
                    //     current_pos = 0;
                    // }
                    thread::sleep(Duration::from_millis(150));
                }
            }
            // channel closes when tx dropped
            eyre::Ok(())
        })
        .wrap_err("Failed to spawn win-file-content-watch thread")?;

    Ok(rx)
}
