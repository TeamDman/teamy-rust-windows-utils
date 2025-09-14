use crate::console::ctrl_handler;
use crate::console::enable_ansi_support;
use eyre::Context;
use tracing::info;
use windows::Win32::System::Console::AllocConsole;
use windows::Win32::System::Console::SetConsoleCtrlHandler;

pub fn console_create() -> eyre::Result<()> {
    // Create new console
    unsafe { AllocConsole() }.wrap_err("Failed to allocate console")?;

    // Attach ctrl+c handler
    unsafe { SetConsoleCtrlHandler(Some(ctrl_handler), true) }
        .wrap_err("Failed to set console control handler")?;

    // Enable ANSI support
    enable_ansi_support().wrap_err("Failed to enable ANSI support")?;

    
    // Tell the user whats up
    info!("Console allocated, new logs will be visible here.");
    info!("Closing this window will exit the program.");
    Ok(())
}
