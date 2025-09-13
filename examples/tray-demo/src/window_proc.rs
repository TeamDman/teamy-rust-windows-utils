use teamy_rust_windows_utils::console::get_our_hwnd;
use teamy_rust_windows_utils::tray::delete_tray_icon;
use teamy_rust_windows_utils::window::WindowUserData;
use teamy_rust_windows_utils::window::clear_window_user_data;
use tracing::debug;
use tracing::error;
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
    match message {
        WM_CLOSE => {
            let hwnd = get_our_hwnd();
            // Clean up the tray icon before closing
            if let Err(e) = delete_tray_icon(hwnd) {
                error!("Failed to delete tray icon: {}", e);
            }
            unsafe { DestroyWindow(hwnd) }.ok();
            LRESULT(0)
        }
        WM_DESTROY => {
            let hwnd = get_our_hwnd();
            // Clean up the tray icon before quitting
            if let Err(e) = delete_tray_icon(hwnd) {
                debug!("Failed to delete tray icon, this always happens :P {}", e);
            }
            unsafe { PostQuitMessage(0) };

            LRESULT(0)
        }
        _ => {
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }
    }
}
