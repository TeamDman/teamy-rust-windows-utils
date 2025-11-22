use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::core::BOOL;

#[derive(Debug)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub title: String,
    pub class_name: String,
    pub rect: RECT,
    pub process_id: u32,
    pub thread_id: u32,
    pub is_visible: bool,
}

pub fn enumerate_windows() -> eyre::Result<Vec<WindowInfo>> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(Some(enum_window_proc), LPARAM(&mut windows as *mut _ as _))?;
    }
    Ok(windows)
}

unsafe extern "system" fn enum_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };

    // Get Title
    let mut title_buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
    let title = String::from_utf16_lossy(&title_buf[..len as usize]);

    // Get Class Name
    let mut class_buf = [0u16; 512];
    let len = unsafe { GetClassNameW(hwnd, &mut class_buf) };
    let class_name = String::from_utf16_lossy(&class_buf[..len as usize]);

    // Get Rect
    let mut rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut rect) };

    // Get PID/TID
    let mut process_id = 0;
    let thread_id = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };

    // Get Visibility
    let is_visible = unsafe { IsWindowVisible(hwnd) }.as_bool();

    windows.push(WindowInfo {
        hwnd,
        title,
        class_name,
        rect,
        process_id,
        thread_id,
        is_visible,
    });

    BOOL(1)
}
