use eyre::bail;
use tracing::debug;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::SetLastError;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;

pub trait WindowUserData: 'static {
    /// Return true if message was handled, false to call DefWindowProc
    fn handle(message: u32, wparam: WPARAM, lparam: LPARAM) -> bool;
}

#[track_caller]
pub fn set_window_user_data<T: WindowUserData>(hwnd: HWND, data: T) -> eyre::Result<()> {
    debug!(
        "Setting window user data for hwnd={:?} from {}",
        hwnd,
        std::panic::Location::caller()
    );
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let rtn = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(Box::new(data)) as _) };
    if rtn == 0 {
        let cause = windows::core::Error::from_thread();
        if cause.code().0 == 0 {
            // This means the previous value was actually zero, so no error occurred
            Ok(())
        } else {
            bail!(eyre::eyre!("Failed to set window user data").wrap_err(cause));
        }
    } else {
        // Free previous pointer to avoid leaking when replacing
        let _ = unsafe { Box::from_raw(rtn as *mut T) };
        Ok(())
    }
}

#[track_caller]
pub fn get_window_user_data<T: WindowUserData>(hwnd: HWND) -> eyre::Result<&'static mut T> {
    debug!(
        "Getting window user data for hwnd={:?} from {}",
        hwnd,
        std::panic::Location::caller()
    );
    let user_data = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if user_data == 0 {
        let cause = windows::core::Error::from_thread();
        bail!(eyre::eyre!("No window user data present").wrap_err(cause));
    } else {
        Ok(unsafe { &mut *(user_data as *mut T) })
    }
}

/// Probably good idea to call this in response to WM_NCDESTROY to avoid leaks
#[track_caller]
pub fn clear_window_user_data<T: WindowUserData>(hwnd: HWND) -> eyre::Result<()> {
    debug!(
        "Clearing window user data for hwnd={:?} from {}",
        hwnd,
        std::panic::Location::caller()
    );
    unsafe { SetLastError(WIN32_ERROR(0)) };
    let prev = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
    // If prev == 0 and last error is 0, there was nothing stored; otherwise drop previous
    if prev != 0 {
        let _ = unsafe { Box::from_raw(prev as *mut T) };
    }
    Ok(())
}
