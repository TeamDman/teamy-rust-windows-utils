use eyre::Context;
use tracing::info;
use windows::Win32::System::Console::FreeConsole;
use crate::console::unbind_and_close_std_handles_for_detach;

pub fn console_detach() -> eyre::Result<()> {
    info!(
        "Detaching from this console, ctrl+c will no longer work for this console until reattached."
    );
    // Reset std handles and close them to avoid keeping the console window alive
    unbind_and_close_std_handles_for_detach();
    unsafe { FreeConsole() }.wrap_err("Failed to free console")?;

    _ = crate::console::check_inheriting::is_inheriting_console(); // for logging

    Ok(())
}
