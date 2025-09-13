use teamy_rust_windows_utils::window::WindowUserData;
use teamy_rust_windows_utils::window::get_window_user_data;
use teamy_rust_windows_utils::window::set_window_user_data;
use tracing::debug;
use tracing::instrument;
use tracing::warn;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

#[instrument]
pub unsafe extern "system" fn window_proc<T: WindowUserData>(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            // Ensure we have some instance of the user data for  our custom message handling
            let data = T::new_from_hwnd(hwnd);
            set_window_user_data(hwnd, data).unwrap_or_else(|e| {
                warn!("Failed to set initial window user data: {e}");
            });
            return LRESULT(0);
        }
        _ => match get_window_user_data::<T>(hwnd) {
            Err(e) => {
                warn!("No user data present! (error: {e}) Deferring to DefWindowProc",);
                return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
            }
            Ok(user_data) => {
                debug!("Found user data: {:?}", user_data as *const T);
                if user_data.handle(message, wparam, lparam) {
                    return LRESULT(0);
                } else {
                    return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
                }
            }
        },
    }
}
