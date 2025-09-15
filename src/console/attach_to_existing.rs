use crate::console::console_detach;
use eyre::Context;
use tracing::Level;
use tracing::info;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::System::Console::*;
use windows::core::w;

/// If called by a new process attaching to an existing process,
/// this should be called before stdout/stderr usage to avoid loss of logs.
///
/// See also: [`ATTACH_PARENT_PROCESS`]
pub fn console_attach(pid: u32) -> eyre::Result<()> {
    let debug_logs_enabled = tracing::event_enabled!(Level::DEBUG);
    if debug_logs_enabled {
        eprintln!("Reusing console with PID: {pid}");
    }

    unsafe {
        // Detach from (non-existent) default console just in case
        let _ = console_detach();

        // Try to attach to the parent console
        AttachConsole(pid)
            .wrap_err_with(|| format!("Failed to attach to console with PID {pid}."))?;

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

    if pid == ATTACH_PARENT_PROCESS {
        info!("Attached to parent process console");
    } else {
        info!("Attached to console with PID: {pid}", pid = pid.to_string());
    }
    Ok(())
}
