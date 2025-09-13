use windows::Win32::UI::WindowsAndMessaging::{HICON, IDI_APPLICATION};

use crate::hicon::get_icon_from_current_module;

pub fn get_application_icon() -> eyre::Result<HICON> {
    get_icon_from_current_module(IDI_APPLICATION)
}
