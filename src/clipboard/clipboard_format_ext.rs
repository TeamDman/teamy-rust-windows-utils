use std::borrow::Cow;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::System::DataExchange::GetClipboardFormatNameW;
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
use windows::Win32::System::Ole::CLIPBOARD_FORMAT;

pub trait ClipboardFormatExt {
    fn display(&self) -> Cow<'_, str>;
}

impl ClipboardFormatExt for CLIPBOARD_FORMAT {
    fn display(&self) -> Cow<'_, str> {
        let mut buffer = vec![0u16; 256];
        let len = unsafe { GetClipboardFormatNameW(self.0 as u32, buffer.as_mut_slice()) };
        if len > 0 {
            let name = OsString::from_wide(&buffer[..len as usize])
                .to_string_lossy()
                .into_owned();
            return Cow::Owned(name);
        }

        match *self {
            CF_TEXT => Cow::Borrowed("CF_TEXT"),
            CF_BITMAP => Cow::Borrowed("CF_BITMAP"),
            CF_METAFILEPICT => Cow::Borrowed("CF_METAFILEPICT"),
            CF_SYLK => Cow::Borrowed("CF_SYLK"),
            CF_DIF => Cow::Borrowed("CF_DIF"),
            CF_TIFF => Cow::Borrowed("CF_TIFF"),
            CF_OEMTEXT => Cow::Borrowed("CF_OEMTEXT"),
            CF_DIB => Cow::Borrowed("CF_DIB"),
            CF_PALETTE => Cow::Borrowed("CF_PALETTE"),
            CF_PENDATA => Cow::Borrowed("CF_PENDATA"),
            CF_RIFF => Cow::Borrowed("CF_RIFF"),
            CF_WAVE => Cow::Borrowed("CF_WAVE"),
            CF_UNICODETEXT => Cow::Borrowed("CF_UNICODETEXT"),
            CF_ENHMETAFILE => Cow::Borrowed("CF_ENHMETAFILE"),
            CF_HDROP => Cow::Borrowed("CF_HDROP"),
            CF_LOCALE => Cow::Borrowed("CF_LOCALE"),
            CF_DIBV5 => Cow::Borrowed("CF_DIBV5"),
            _ => Cow::Owned(format!("Unknown format (0x{:X})", self.0)),
        }
    }
}
