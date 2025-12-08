use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::RECT;
use windows::Win32::System::Threading::OpenProcess;
use windows::Win32::System::Threading::PROCESS_NAME_WIN32;
use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
use windows::Win32::System::Threading::QueryFullProcessImageNameW;
use windows::Win32::UI::WindowsAndMessaging::EnumWindows;
use windows::Win32::UI::WindowsAndMessaging::GW_OWNER;
use windows::Win32::UI::WindowsAndMessaging::GWL_EXSTYLE;
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
use windows::Win32::UI::WindowsAndMessaging::GetWindow;
use windows::Win32::UI::WindowsAndMessaging::GetWindowLongW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::IsWindowVisible;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_APPWINDOW;
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOOLWINDOW;
use windows::core::BOOL;
use windows::core::Owned;
use windows::core::PWSTR;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WindowInfo {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_hwnd"))]
    pub hwnd: HWND,
    pub title: String,
    pub class_name: String,
    pub exe_path: String,
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_rect"))]
    pub rect: RECT,
    pub process_id: u32,
    pub thread_id: u32,
    pub is_visible: bool,
    pub is_on_taskbar: bool,
}

#[cfg(feature = "serde")]
fn serialize_hwnd<S>(hwnd: &HWND, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(hwnd.0 as u64)
}

#[cfg(feature = "serde")]
fn serialize_rect<S>(rect: &RECT, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;
    let mut state = serializer.serialize_struct("RECT", 4)?;
    state.serialize_field("left", &rect.left)?;
    state.serialize_field("top", &rect.top)?;
    state.serialize_field("right", &rect.right)?;
    state.serialize_field("bottom", &rect.bottom)?;
    state.end()
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

    let mut exe_path = String::new();
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) };
    if let Ok(handle) = handle {
        let handle = unsafe { Owned::new(handle) };
        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;
        if unsafe {
            QueryFullProcessImageNameW(
                *handle,
                PROCESS_NAME_WIN32,
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            )
            .is_ok()
        } {
            exe_path = String::from_utf16_lossy(&buffer[..size as usize]);
        }
    }

    // Get Visibility
    let is_visible = unsafe { IsWindowVisible(hwnd) }.as_bool();

    // Check if on Taskbar
    let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
    let owner = unsafe { GetWindow(hwnd, GW_OWNER) }.unwrap_or_default();

    let is_app_window = (ex_style & WS_EX_APPWINDOW.0) != 0;
    let is_tool_window = (ex_style & WS_EX_TOOLWINDOW.0) != 0;

    let is_on_taskbar = if !is_visible {
        false
    } else if is_app_window {
        true
    } else if is_tool_window {
        false
    } else {
        owner.0.is_null()
    };

    windows.push(WindowInfo {
        hwnd,
        title,
        class_name,
        exe_path,
        rect,
        process_id,
        thread_id,
        is_visible,
        is_on_taskbar,
    });

    BOOL(1)
}
