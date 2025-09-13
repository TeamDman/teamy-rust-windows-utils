use crate::tray::ID_TRAYICON;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::NIM_DELETE;
use windows::Win32::UI::Shell::NOTIFYICONDATAW;
use windows::Win32::UI::Shell::Shell_NotifyIconW;

pub fn delete_tray_icon(hwnd: HWND) -> eyre::Result<()> {
    let notify_icon_data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID_TRAYICON,
        ..Default::default()
    };

    // Remove the icon from the system tray
    unsafe { Shell_NotifyIconW(NIM_DELETE, &notify_icon_data).ok() }?;

    Ok(())
}
