use tracing::error;
use tracing::info;
use windows::Win32::System::Console::FreeConsole;

pub fn detach_console() {
    unsafe {
        info!(
            "Detaching from this console, ctrl+c will no longer work and you will have to use the system tray icon to close the program"
        );
        if let Err(e) = FreeConsole() {
            error!("Failed to free console: {}", e);
        }
    }
}
