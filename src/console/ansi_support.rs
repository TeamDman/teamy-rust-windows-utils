use eyre::Context;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
use windows::Win32::System::Console::GetConsoleMode;
use windows::Win32::System::Console::GetStdHandle;
use windows::Win32::System::Console::STD_OUTPUT_HANDLE;
use windows::Win32::System::Console::SetConsoleMode;

pub fn enable_ansi_support() -> eyre::Result<()> {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE).wrap_err("Failed to get standard output handle")?;
        if handle == HANDLE::default() {
            return Err(windows::core::Error::from_thread())
                .wrap_err("Got standard output handle, but it was invalid");
        }

        let mut mode = std::mem::zeroed();
        GetConsoleMode(handle, &mut mode).wrap_err("Failed to get console mode")?;
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING)
            .wrap_err("Failed to set console mode")?;
        Ok(())
    }
}
