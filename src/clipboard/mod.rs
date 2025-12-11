//! <https://learn.microsoft.com/en-us/windows/win32/shell/clipboard>

mod clipboard_format_ext;
mod clipboard_guard;
mod clipboard_io;

pub use clipboard_format_ext::*;
pub use clipboard_guard::*;
pub use clipboard_io::*;
