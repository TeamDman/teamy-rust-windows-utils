use eyre::Context;
use eyre::Result;
use windows::Win32::System::DataExchange::CloseClipboard;
use windows::Win32::System::DataExchange::OpenClipboard;

pub struct ClipboardGuard;

impl ClipboardGuard {
    pub fn open() -> Result<Self> {
        unsafe { OpenClipboard(None) }.wrap_err("Failed to open clipboard")?;
        Ok(Self)
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}
