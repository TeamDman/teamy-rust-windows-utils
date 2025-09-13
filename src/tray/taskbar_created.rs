use std::sync::OnceLock;
use windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
use windows::core::w;

/// Returns the atom/message ID for the "TaskbarCreated" broadcast message.
/// Explorer broadcasts this after the taskbar is (re)created, e.g., after a crash.
/// Apps should re-add their tray icons when they receive this message.
pub fn get_or_register_taskbar_created_message() -> u32 {
    static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
    *TASKBAR_CREATED.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}
