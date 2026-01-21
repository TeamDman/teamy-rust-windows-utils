use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;
use std::path::PathBuf;

use super::gui;

/// Browse for icons in DLL files.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct IconBrowseArgs {
    /// Paths to DLL files to browse for icons. If not provided, defaults to mmres.dll.
    #[arg()]
    pub paths: Vec<PathBuf>,
}

impl IconBrowseArgs {
    pub fn invoke(self) -> Result<()> {
        let paths = if self.paths.is_empty() {
            let system_root =
                std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
            vec![PathBuf::from(format!("{system_root}\\system32\\mmres.dll"))]
        } else {
            self.paths
        };
        gui::run_icon_browser(paths)
    }
}

impl ToArgs for IconBrowseArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.paths.iter().map(|p| p.as_os_str().to_owned()).collect()
    }
}
