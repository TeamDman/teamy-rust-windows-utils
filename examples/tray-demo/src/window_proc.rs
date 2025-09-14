use teamy_rust_windows_utils::tray::TRAY_ICON_ID;
use teamy_rust_windows_utils::tray::WM_TASKBAR_CREATED;
use teamy_rust_windows_utils::tray::WM_USER_TRAY_CALLBACK;
use teamy_rust_windows_utils::tray::delete_tray_icon;
use teamy_rust_windows_utils::tray::re_add_tray_icon;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
use windows::Win32::UI::WindowsAndMessaging::WM_DESTROY;
use windows::Win32::UI::WindowsAndMessaging::*;

#[instrument]
pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Cache the broadcast message atom once per process; cheap on subsequent calls.
    // If you prefer, this can be hoisted into a static in this module too.
    match message {
        // Tray icon callback message (set via NOTIFYICONDATAW.uCallbackMessage = WM_USER + 1)
        WM_USER_TRAY_CALLBACK  => {
            match lparam.0 as u32 {
                WM_LBUTTONDOWN => info!("Tray icon left button down"),
                WM_LBUTTONUP => info!("Tray icon left button up"),
                WM_LBUTTONDBLCLK => info!("Tray icon left button double click"),
                WM_RBUTTONDOWN => info!("Tray icon right button down"),
                WM_RBUTTONUP => info!("Tray icon right button up"),
                WM_RBUTTONDBLCLK => info!("Tray icon right button double click"),
                WM_CONTEXTMENU => info!("Tray icon context menu"),
                WM_MOUSEMOVE => { /* ignore mouse move */ }
                x => info!("Tray icon unknown event: {x}"),
            }
            LRESULT(0)
        }
        m if m == *WM_TASKBAR_CREATED => {
            // Explorer/taskbar restarted; re-add our tray icon
            if let Err(e) = re_add_tray_icon() {
                error!("Failed to re-add tray icon after TaskbarCreated: {}", e);
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            // Clean up the tray icon before closing
            if let Err(e) = delete_tray_icon(hwnd) {
                error!("Failed to delete tray icon: {}", e);
            }
            unsafe { DestroyWindow(hwnd) }.ok();
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => {
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
    }
}
