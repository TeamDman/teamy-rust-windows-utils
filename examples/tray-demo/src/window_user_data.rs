use teamy_rust_windows_utils::window::WindowUserData;
use tracing::debug;
use tracing::error;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Shell::NIM_DELETE;
use windows::Win32::UI::Shell::NOTIFYICONDATAW;
use windows::Win32::UI::Shell::Shell_NotifyIconW;
use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;

pub struct MyWindowUserData {
    pub hwnd: HWND,
    pub notify_icon_data: NOTIFYICONDATAW,
}
impl WindowUserData for MyWindowUserData {
    fn new_from_hwnd(hwnd: HWND) -> Self {
        Self {
            hwnd,
            notify_icon_data: Default::default(),
        }
    }
    fn new_from_hwnd_and_notify_icon_data(hwnd: HWND, notify_icon_data: NOTIFYICONDATAW) -> Self {
        Self {
            hwnd,
            notify_icon_data,
        }
    }
    fn handle(&self, message: u32, _wparam: WPARAM, _lparam: LPARAM) -> bool {
        match message {
            WM_CLOSE => {
                unsafe {
                    // Clean up the tray icon before closing
                    if let Err(e) = Shell_NotifyIconW(NIM_DELETE, &self.notify_icon_data).ok() {
                        error!("Failed to delete tray icon: {}", e);
                    }
                    DestroyWindow(self.hwnd).ok();
                }
                true
            }
            WM_DESTROY => {
                unsafe {
                    // Clean up the tray icon before quitting
                    if let Err(e) = Shell_NotifyIconW(NIM_DELETE, &self.notify_icon_data).ok() {
                        debug!("Failed to delete tray icon, this always happens :P {}", e);
                    }
                    PostQuitMessage(0);
                }
                true
            }
            _ => {
                debug!(
                    "Unhandled message in MyWindowUserData: message={message}, wparam={:?}, lparam={:?}",
                    _wparam, _lparam
                );
                false
            }
        }
    }
}
