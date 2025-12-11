use crate::cli::to_args::ToArgs;
use crate::clipboard::ClipboardFormatExt;
use crate::clipboard::ClipboardGuard;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Context;
use eyre::Result;
use std::convert::TryFrom;
use std::ffi::CStr;
use std::ffi::OsString;
use std::os::raw::c_char;
use std::os::windows::ffi::OsStringExt;
use widestring::U16CStr;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::DataExchange::EnumClipboardFormats;
use windows::Win32::System::DataExchange::GetClipboardData;
use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
use windows::Win32::System::Memory::GlobalLock;
use windows::Win32::System::Memory::GlobalSize;
use windows::Win32::System::Memory::GlobalUnlock;
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::System::Ole::CF_OEMTEXT;
use windows::Win32::System::Ole::CF_TEXT;
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::System::Ole::CLIPBOARD_FORMAT;
use windows::Win32::UI::Shell::DragQueryFileW;
use windows::Win32::UI::Shell::HDROP;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct ClipboardShowArgs {}

impl ToArgs for ClipboardShowArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}

impl ClipboardShowArgs {
    pub fn invoke(self) -> Result<()> {
        let description = describe_clipboard_contents()?;
        println!("{}", description);
        Ok(())
    }
}

pub fn describe_clipboard_contents() -> Result<String> {
    let _guard = ClipboardGuard::open().wrap_err("Failed to open clipboard")?;

    let mut description = String::new();

    // If the clipboard currently contains drag-and-drop data, list the file paths.
    if unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok() } {
        let file_data = unsafe { GetClipboardData(CF_HDROP.0 as u32)? };
        if !file_data.is_invalid() {
            let hdrop = HDROP(file_data.0);
            let file_count = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
            description.push_str(&format!("Found {} files in clipboard:\n", file_count));

            for i in 0..file_count {
                let mut buffer = vec![0u16; MAX_PATH as usize];
                let len = unsafe { DragQueryFileW(hdrop, i, Some(buffer.as_mut_slice())) };
                if len > 0 {
                    let path = OsString::from_wide(&buffer[..len as usize]);
                    description.push_str(&format!("- {}\n", path.to_string_lossy()));
                }
            }
        }
    }

    let mut format = 0;
    loop {
        let next_format = unsafe { EnumClipboardFormats(format) };
        if next_format == 0 {
            let error = unsafe { GetLastError() };
            if error != ERROR_SUCCESS {
                description.push_str(&format!("\nEnumClipboardFormats error: {}\n", error.0));
            }
            break;
        }

        format = next_format;
        let format_name = CLIPBOARD_FORMAT(u16::try_from(format)?);
        description.push_str(&format!(
            "\nFormat: {} (0x{:X})\n",
            format_name.display(),
            format
        ));

        let data_handle = unsafe { GetClipboardData(format)? };
        if data_handle.is_invalid() {
            continue;
        }

        // Wrap the raw clipboard handle so GlobalLock/GlobalSize can operate on it.
        let hglobal = HGLOBAL(data_handle.0);

        let content = match format {
            x if x == CF_TEXT.0 as u32 => read_clipboard_ascii(hglobal),
            x if x == CF_OEMTEXT.0 as u32 => read_clipboard_ascii(hglobal),
            x if x == CF_UNICODETEXT.0 as u32 => read_clipboard_unicode(hglobal),
            _ => {
                // Fallback for unknown formats: report the raw buffer length.
                let size = unsafe { GlobalSize(hglobal) } as usize;
                format!("[Binary data, {} bytes]", size)
            }
        };

        description.push_str(&format!("Content: {}\n", content));
    }

    Ok(description)
}

fn read_clipboard_ascii(handle: HGLOBAL) -> String {
    // Lock the global handle so we can read the raw bytes safely.
    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        return "[Failed to lock clipboard data]".into();
    }

    let data_ptr = lock as *const u8;
    let c_str = unsafe { CStr::from_ptr(data_ptr as *const c_char) };
    let result = String::from_utf8_lossy(c_str.to_bytes()).to_string();
    let _ = unsafe { GlobalUnlock(handle) };
    result
}

fn read_clipboard_unicode(handle: HGLOBAL) -> String {
    // Lock the clipboard handle and interpret it as UTF-16 data.
    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        return "[Failed to lock clipboard data]".into();
    }

    let data_ptr = lock as *const u16;
    let wide_cstr = unsafe { U16CStr::from_ptr_str(data_ptr) };
    let result = wide_cstr.to_string_lossy().to_string();
    let _ = unsafe { GlobalUnlock(handle) };
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_describe_clipboard_contents() {
        let description = describe_clipboard_contents().unwrap();
        println!("{}", description);
    }
}
