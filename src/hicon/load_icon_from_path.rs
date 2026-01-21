use crate::audio::TeamyImmDeviceIcon;
use crate::hicon::hicon_to_rgba;
use crate::string::EasyPCWSTR;
use eyre::bail;
use eyre::ensure;
use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::LoadLibraryW;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::IMAGE_ICON;
use windows::Win32::UI::WindowsAndMessaging::LR_DEFAULTSIZE;
use windows::Win32::UI::WindowsAndMessaging::LR_LOADFROMFILE;
use windows::Win32::UI::WindowsAndMessaging::LR_SHARED;
use windows::Win32::UI::WindowsAndMessaging::LoadImageW;
use windows::core::Owned;

pub fn load_icon_from_path(path: &str) -> eyre::Result<TeamyImmDeviceIcon> {
    // may be a problem with this lol
    // 7 months ago the code I am rewriting this from had this as the commit message
    // > fallback mic icon logic partially working, it's grabbing the line in icon instead of the mic icon

    match path.split(",-").collect::<Vec<_>>().as_slice() {
        [path] if path.to_ascii_lowercase().ends_with(".ico") => {
            // Load the image handle
            let handle = unsafe {
                LoadImageW(
                    None,
                    path.easy_pcwstr()?.as_ref(),
                    IMAGE_ICON,
                    0,
                    0,
                    LR_DEFAULTSIZE | LR_SHARED | LR_LOADFROMFILE,
                )
            }?;
            ensure!(!handle.is_invalid());

            // Convert the image
            unsafe { hicon_to_rgba(HICON(handle.0)).map(TeamyImmDeviceIcon::new) }
        }
        [path, index_str] => {
            let path = path.strip_prefix("@").unwrap_or(path);
            let index: u16 = index_str.parse()?;

            // Load the module
            let hmodule = unsafe { LoadLibraryW(path.easy_pcwstr()?.as_ref()) }?;
            ensure!(!hmodule.is_invalid());
            let hmodule = unsafe { Owned::new(hmodule) };

            // Somewhere it is mentioned that macros are out of scope of the windows-rs project
            #[allow(non_snake_case)]
            pub fn MAKEINTRESOURCEW(i: u16) -> windows::core::PCWSTR {
                windows::core::PCWSTR(i as usize as *const u16)
            }

            // Load the image handle
            let image_handle = unsafe {
                LoadImageW(
                    Some(HINSTANCE::from(*hmodule)),
                    MAKEINTRESOURCEW(index),
                    IMAGE_ICON,
                    0,
                    0,
                    LR_DEFAULTSIZE | LR_SHARED,
                )
            }?;
            ensure!(!image_handle.is_invalid());

            // Convert the image
            unsafe { hicon_to_rgba(HICON(image_handle.0)).map(TeamyImmDeviceIcon::new) }
        }
        _ => {
            bail!(
                "Invalid icon path format: expected 'path.ico' or 'path,-index', got '{}'",
                path
            );
        }
    }
}
