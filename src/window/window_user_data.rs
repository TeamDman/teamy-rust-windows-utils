use eyre::bail;
use tracing::debug;
use windows::core::HRESULT;
use windows::Win32::Foundation::SetLastError;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Shell::NOTIFYICONDATAW;
use windows::Win32::UI::WindowsAndMessaging::GWLP_USERDATA;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW;
use windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW;

pub trait WindowUserData: 'static {
    fn new_from_hwnd(hwnd: HWND) -> Self;
    fn new_from_hwnd_and_notify_icon_data(hwnd: HWND, notify_icon_data: NOTIFYICONDATAW) -> Self;

    /// Return true if message was handled, false to call DefWindowProc
    fn handle(&self, message: u32, wparam: WPARAM, lparam: LPARAM) -> bool;
}


#[track_caller]
pub fn set_window_user_data<T: WindowUserData>(hwnd: HWND, data: T) -> eyre::Result<()> {
    debug!("Setting window user data for hwnd={:?} from {}", hwnd, std::panic::Location::caller());
    unsafe {
        SetLastError(WIN32_ERROR(0));
        let rtn = SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(Box::new(data)) as _);
        if rtn == 0 {
            let cause = windows::core::Error::from_thread();
            if cause.code().0 == 0 {
                // This means the previous value was actually zero, so no error occurred
                return Ok(());
            } else {
                bail!(eyre::eyre!("Failed to set window user data").wrap_err(cause));
            }
        } else {
            Ok(())
        }
    }
}

#[track_caller]
pub fn get_window_user_data<T: WindowUserData>(hwnd: HWND) -> eyre::Result<&'static mut T> {
    debug!("Getting window user data for hwnd={:?} from {}", hwnd, std::panic::Location::caller());
    let user_data = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if user_data == 0 {
        let cause = windows::core::Error::from_thread();
        bail!(eyre::eyre!("No window user data present").wrap_err(cause));
    } else {
        Ok(unsafe { &mut *(user_data as *mut T) })
    }
}
