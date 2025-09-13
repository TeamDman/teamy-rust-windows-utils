use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;
use windows::core::Param;
use windows::core::ParamValue;

const WM_TRAYICON: u32 = WM_USER + 1;
pub const ID_TRAYICON: u32 = 1;

pub fn add_tray_icon(
    hwnd: HWND,
    icon: HICON,
    tooltip: impl Param<PCWSTR>,
) -> eyre::Result<NOTIFYICONDATAW> {
    // Create tray icon
    let mut notify_icon_data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID_TRAYICON,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        hIcon: icon,
        szTip: [0; 128],
        ..Default::default()
    };

    // Set tooltip
    let tooltip: ParamValue<PCWSTR> = unsafe { tooltip.param() };
    let tooltip = tooltip.abi();
    let tooltip = unsafe { tooltip.as_wide() };
    notify_icon_data.szTip[..tooltip.len()].copy_from_slice(tooltip);

    // Add the icon to the system tray
    unsafe { Shell_NotifyIconW(NIM_ADD, &notify_icon_data).ok() }?;

    Ok(notify_icon_data)
}
