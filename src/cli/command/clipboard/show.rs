use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Context;
use eyre::Result;
use std::ffi::CStr;
use std::ffi::OsString;
use std::os::raw::c_char;
use std::os::windows::ffi::OsStringExt;
use widestring::WideCStr;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::System::DataExchange::CloseClipboard;
use windows::Win32::System::DataExchange::EnumClipboardFormats;
use windows::Win32::System::DataExchange::GetClipboardData;
use windows::Win32::System::DataExchange::GetClipboardFormatNameW;
use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
use windows::Win32::System::DataExchange::OpenClipboard;
use windows::Win32::System::Memory::GlobalLock;
use windows::Win32::System::Memory::GlobalSize;
use windows::Win32::System::Memory::GlobalUnlock;
use windows::Win32::System::Ole::CF_BITMAP;
use windows::Win32::System::Ole::CF_DIB;
use windows::Win32::System::Ole::CF_DIBV5;
use windows::Win32::System::Ole::CF_DIF;
use windows::Win32::System::Ole::CF_ENHMETAFILE;
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::System::Ole::CF_LOCALE;
use windows::Win32::System::Ole::CF_METAFILEPICT;
use windows::Win32::System::Ole::CF_OEMTEXT;
use windows::Win32::System::Ole::CF_PALETTE;
use windows::Win32::System::Ole::CF_PENDATA;
use windows::Win32::System::Ole::CF_RIFF;
use windows::Win32::System::Ole::CF_SYLK;
use windows::Win32::System::Ole::CF_TEXT;
use windows::Win32::System::Ole::CF_TIFF;
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::System::Ole::CF_WAVE;
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
        let format_name = get_clipboard_format_name(format)?;
        description.push_str(&format!("\nFormat: {} (0x{:X})\n", format_name, format));

        let data_handle = unsafe { GetClipboardData(format)? };
        if data_handle.is_invalid() {
            continue;
        }

        let hglobal = HGLOBAL(data_handle.0);

        let content = match format {
            x if x == CF_TEXT.0 as u32 => read_clipboard_ascii(hglobal),
            x if x == CF_OEMTEXT.0 as u32 => read_clipboard_ascii(hglobal),
            x if x == CF_UNICODETEXT.0 as u32 => read_clipboard_unicode(hglobal),
            _ => {
                let size = unsafe { GlobalSize(hglobal) } as usize;
                format!("[Binary data, {} bytes]", size)
            }
        };

        description.push_str(&format!("Content: {}\n", content));
    }

    Ok(description)
}

struct ClipboardGuard;

impl ClipboardGuard {
    fn open() -> Result<Self> {
        unsafe { OpenClipboard(None) }.wrap_err("Failed to open clipboard")?;
        Ok(Self)
    }
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseClipboard() };
    }
}

fn read_clipboard_ascii(handle: HGLOBAL) -> String {
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
    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        return "[Failed to lock clipboard data]".into();
    }

    let data_ptr = lock as *const u16;
    let wide_cstr = unsafe { WideCStr::from_ptr_str(data_ptr) };
    let result = wide_cstr.to_string_lossy().to_string();
    let _ = unsafe { GlobalUnlock(handle) };
    result
}
fn get_clipboard_format_name(format: u32) -> Result<String> {
    let mut buffer = vec![0u16; 256];
    let len = unsafe { GetClipboardFormatNameW(format, buffer.as_mut_slice()) };
    if len > 0 {
        return Ok(OsString::from_wide(&buffer[..len as usize])
            .to_string_lossy()
            .to_string());
    }

    let known = match format {
        x if x == CF_TEXT.0 as u32 => "CF_TEXT",
        x if x == CF_BITMAP.0 as u32 => "CF_BITMAP",
        x if x == CF_METAFILEPICT.0 as u32 => "CF_METAFILEPICT",
        x if x == CF_SYLK.0 as u32 => "CF_SYLK",
        x if x == CF_DIF.0 as u32 => "CF_DIF",
        x if x == CF_TIFF.0 as u32 => "CF_TIFF",
        x if x == CF_OEMTEXT.0 as u32 => "CF_OEMTEXT",
        x if x == CF_DIB.0 as u32 => "CF_DIB",
        x if x == CF_PALETTE.0 as u32 => "CF_PALETTE",
        x if x == CF_PENDATA.0 as u32 => "CF_PENDATA",
        x if x == CF_RIFF.0 as u32 => "CF_RIFF",
        x if x == CF_WAVE.0 as u32 => "CF_WAVE",
        x if x == CF_UNICODETEXT.0 as u32 => "CF_UNICODETEXT",
        x if x == CF_ENHMETAFILE.0 as u32 => "CF_ENHMETAFILE",
        x if x == CF_HDROP.0 as u32 => "CF_HDROP",
        x if x == CF_LOCALE.0 as u32 => "CF_LOCALE",
        x if x == CF_DIBV5.0 as u32 => "CF_DIBV5",
        _ => {
            return Err(eyre::eyre!(
                "Failed to get clipboard format name for {}",
                format
            ));
        }
    };

    Ok(known.to_string())
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
