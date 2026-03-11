use crate::event_loop::run_message_loop;
use crate::module::get_current_module;
use crate::string::EasyPCWSTR;
use eyre::{Result, WrapErr, bail};
use std::sync::OnceLock;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, PostQuitMessage, RegisterClassExW,
    SW_SHOW, ShowWindow, WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WNDCLASSEXW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::w;

const BASIC_WINDOW_CLASS_NAME: windows::core::PCWSTR = w!("TeamyWindowsBasicWindow");

pub fn create_basic_window(title: &str) -> Result<HWND> {
    ensure_basic_window_class_registered()?;

    let instance = get_current_module()?;
    let title = title.easy_pcwstr()?;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            BASIC_WINDOW_CLASS_NAME,
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            600,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .wrap_err("Failed to create basic window")?;

    let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };

    Ok(hwnd)
}

pub fn run_basic_window(title: &str) -> Result<isize> {
    let hwnd = create_basic_window(title)?;
    run_message_loop(Some(hwnd))?;
    Ok(hwnd.0 as isize)
}

fn ensure_basic_window_class_registered() -> Result<()> {
    static REGISTRATION: OnceLock<Result<(), String>> = OnceLock::new();
    match REGISTRATION.get_or_init(|| register_basic_window_class().map_err(|e| format!("{e:#}"))) {
        Ok(()) => Ok(()),
        Err(message) => bail!(message.clone()),
    }
}

fn register_basic_window_class() -> Result<()> {
    let instance = get_current_module()?;
    let window_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(basic_window_proc),
        hInstance: instance.into(),
        lpszClassName: BASIC_WINDOW_CLASS_NAME,
        ..Default::default()
    };

    let atom = unsafe { RegisterClassExW(&window_class) };
    if atom == 0 {
        return Err(windows::core::Error::from_thread()).wrap_err("Failed to register basic window class");
    }

    Ok(())
}

unsafe extern "system" fn basic_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CLOSE => {
            unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd) }.ok();
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
