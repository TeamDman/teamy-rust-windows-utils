use crate::cli::to_args::ToArgs;
use crate::clipboard::write_clipboard;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Context;
use eyre::Result;
use std::ffi::OsString;
use widestring::U16CString;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct ClipboardSetArgs {
    #[arg(value_name = "TEXT")]
    pub value: String,
}

impl ToArgs for ClipboardSetArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![self.value.clone().into()]
    }
}

impl ClipboardSetArgs {
    pub fn invoke(self) -> Result<()> {
        let wide = U16CString::from_str(&self.value)?;
        write_clipboard(wide).wrap_err("Failed to set clipboard text")
    }
}
