use eyre::bail;
use tracing::Level;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Console::*;
use windows::core::w;

use crate::console::console_detach;

/// Reuses the console of the parent process if requested via command line args.
/// This must be called before any logging initialization or stdout/stderr usage.
/// Therefore, the desired log level must be passed in manually.
pub fn console_attach_to_existing(pid: u32, log_level: Level) -> eyre::Result<()> {
    if log_level >= Level::DEBUG {
        eprintln!("Reusing console with PID: {pid}");
    }

    unsafe {
        // Detach from (non-existent) default console just in case
        let _ = console_detach();

        // Try to attach to the parent console
        if let Err(e) = AttachConsole(pid) {
            // If attaching fails, allocate a new console as fallback
            match AllocConsole() {
                Ok(_) => {
                    if log_level >= Level::DEBUG {
                        eprintln!("Failed to attach to console with PID {pid}, allocated a new console instead. Error: {e:?}");
                    }
                }
                Err(e) => {
                    if log_level >= Level::DEBUG {
                        eprintln!("Failed to attach to console with PID {pid}, and failed to allocate a new console. Error: {e:?}");
                    }
                    bail!("Failed to attach to console with PID {pid}, and failed to allocate a new console. Error: {e:?}");
                }
            }
        }

        // Re-open standard handles so Rust's std::io uses the console.
        let con_out = CreateFileW(
            w!("CONOUT$"),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        );
        let con_in = CreateFileW(
            w!("CONIN$"),
            (FILE_GENERIC_READ | FILE_GENERIC_WRITE).0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        );

        if let Ok(con_out) = con_out {
            let _ = SetStdHandle(STD_OUTPUT_HANDLE, con_out);
            let _ = SetStdHandle(STD_ERROR_HANDLE, con_out);

            // Optional: enable ANSI again
            let mut mode = CONSOLE_MODE::default();
            if GetConsoleMode(con_out, &mut mode).is_ok() {
                let _ = SetConsoleMode(
                    con_out,
                    mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT,
                );
            }
        }

        if let Ok(con_in) = con_in {
            let _ = SetStdHandle(STD_INPUT_HANDLE, con_in);
        }
    }
    Ok(())
}
