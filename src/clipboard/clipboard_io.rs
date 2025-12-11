use super::clipboard_guard::ClipboardGuard;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use std::ptr;
use widestring::U16CStr;
use widestring::U16CString;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::DataExchange::EmptyClipboard;
use windows::Win32::System::DataExchange::GetClipboardData;
use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
use windows::Win32::System::DataExchange::SetClipboardData;
use windows::Win32::System::Memory::GMEM_MOVEABLE;
use windows::Win32::System::Memory::GlobalAlloc;
use windows::Win32::System::Memory::GlobalLock;
use windows::Win32::System::Memory::GlobalUnlock;
use windows::Win32::System::Ole::CF_TEXT;
use windows::Win32::System::Ole::CF_UNICODETEXT;

pub fn read_clipboard() -> Result<String> {
    let _guard = ClipboardGuard::open().wrap_err("Failed to open clipboard")?;

    if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT.0 as u32).is_ok() } {
        let handle = unsafe { GetClipboardData(CF_UNICODETEXT.0 as u32)? };
        if handle.is_invalid() {
            bail!("Unicode clipboard handle was invalid");
        }
        read_clipboard_unicode(HGLOBAL(handle.0))
    } else if unsafe { IsClipboardFormatAvailable(CF_TEXT.0 as u32).is_ok() } {
        let handle = unsafe { GetClipboardData(CF_TEXT.0 as u32)? };
        if handle.is_invalid() {
            bail!("ANSI clipboard handle was invalid");
        }
        read_clipboard_ascii(HGLOBAL(handle.0))
    } else {
        bail!("No text data on the clipboard");
    }
}

pub fn write_clipboard(value: impl Into<U16CString>) -> Result<()> {
    let _guard = ClipboardGuard::open().wrap_err("Failed to open clipboard")?;
    unsafe { EmptyClipboard().wrap_err("Failed to empty clipboard")? };

    let wide = value.into();
    let slice = wide.as_slice_with_nul();
    let size = std::mem::size_of_val(slice);
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) }
        .wrap_err("Failed to allocate clipboard buffer")?;
    if handle.is_invalid() {
        bail!("Failed to allocate clipboard buffer");
    }

    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        bail!("Failed to lock clipboard buffer");
    }

    unsafe { ptr::copy_nonoverlapping(slice.as_ptr(), lock as *mut u16, slice.len()) };
    let _ = unsafe { GlobalUnlock(handle) };

    unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(handle.0))) }
        .wrap_err("Failed to set clipboard data")?;

    Ok(())
}

fn read_clipboard_ascii(handle: HGLOBAL) -> Result<String> {
    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        bail!("Failed to lock clipboard data")
    }

    let data_ptr = lock as *const u8;
    let c_str = unsafe { std::ffi::CStr::from_ptr(data_ptr as *const i8) };
    let result = String::from_utf8_lossy(c_str.to_bytes()).to_string();
    let _ = unsafe { GlobalUnlock(handle) };
    Ok(result)
}

fn read_clipboard_unicode(handle: HGLOBAL) -> Result<String> {
    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        bail!("Failed to lock clipboard data")
    }

    let data_ptr = lock as *const u16;
    let wide = unsafe { U16CStr::from_ptr_str(data_ptr) };
    let result = wide.to_string_lossy().to_string();
    let _ = unsafe { GlobalUnlock(handle) };
    Ok(result)
}
