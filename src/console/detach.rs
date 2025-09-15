use eyre::Context;
use tracing::info;
use windows::Win32::System::Console::FreeConsole;

pub fn console_detach() -> eyre::Result<()> {
    info!(
        "Detaching from this console, ctrl+c will no longer work for this console until reattached."
    );
    unsafe { FreeConsole() }.wrap_err("Failed to free console")?;
    Ok(())
}
