use crate::console::attach_ctrl_c_handler;
use crate::console::check_inheriting;
use crate::console::enable_ansi_support;
use eyre::Context;
use tracing::error;
use tracing::info;
use windows::Win32::System::Console::AllocConsole;

pub fn console_create() -> eyre::Result<()> {
    // Create new console
    unsafe { AllocConsole() }.wrap_err("Failed to allocate console")?;

    _ = check_inheriting::is_inheriting_console(); // for logging

    // Attach ctrl+c handler (continue on error)
    if let Err(e) = attach_ctrl_c_handler().wrap_err("Failed to set console control handler") {
        error!("{:?}", e);
    }

    // Enable ANSI support (continue on error)
    if let Err(e) = enable_ansi_support().wrap_err("Failed to enable ANSI support") {
        error!("{:?}", e);
    }

    // Tell the user whats up
    info!("Console allocated, new logs will be visible here.");
    info!("Closing this window will exit the program.");
    Ok(())
}
