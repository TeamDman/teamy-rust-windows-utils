use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::DispatchMessageW;
use windows::Win32::UI::WindowsAndMessaging::GetMessageW;
use windows::Win32::UI::WindowsAndMessaging::MSG;
use windows::Win32::UI::WindowsAndMessaging::TranslateMessage;

/// Pump the message loop for the given window handle, or all windows if None is provided.
pub fn run_message_loop(hwnd: Option<HWND>) -> eyre::Result<()> {
    let mut msg = MSG::default();
    unsafe {
        debug!("Starting message loop");
        while GetMessageW(&mut msg, hwnd, 0, 0).into() {
            _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}
