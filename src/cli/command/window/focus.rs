use crate::cli::to_args::ToArgs;
use crate::window::focus_window;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct WindowFocusArgs {
    /// The HWND of the window to focus
    pub hwnd: isize,
}

impl ToArgs for WindowFocusArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![self.hwnd.to_string().into()]
    }
}

impl WindowFocusArgs {
    pub fn invoke(self) -> Result<()> {
        focus_window(self.hwnd)?;
        Ok(())
    }
}
