use crate::console::console_detach;
use crate::console::enable_ansi_support;
use crate::console::rebind_std_handles_to_console;
use eyre::Context;
use tracing::warn;
use tracing::Level;
use tracing::info;
use windows::Win32::System::Console::*;

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

        // Ensure std handles are bound to the console (reusing shared helper)
        rebind_std_handles_to_console()?;

        // Enable ANSI support (continue on error)
        if let Err(e) = enable_ansi_support() {
            warn!("Failed to enable ANSI support: {:?}", e);
        }
    }

    if pid == ATTACH_PARENT_PROCESS {
        info!("Attached to parent process console");
    } else {
        info!("Attached to console with PID: {pid}", pid = pid.to_string());
    }
    Ok(())
}
