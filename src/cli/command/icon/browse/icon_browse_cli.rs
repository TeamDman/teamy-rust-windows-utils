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
            let system32 = format!("{system_root}\\system32");
            vec![
                PathBuf::from(format!("{system32}\\mmres.dll")),        // Audio/multimedia icons
                PathBuf::from(format!("{system32}\\shell32.dll")),      // Shell icons (folders, files, etc)
                PathBuf::from(format!("{system32}\\imageres.dll")),     // Modern Windows icons
                PathBuf::from(format!("{system32}\\ddores.dll")),       // Device icons
                PathBuf::from(format!("{system32}\\netshell.dll")),     // Network icons
                PathBuf::from(format!("{system32}\\wmploc.dll")),       // Windows Media Player icons
                PathBuf::from(format!("{system32}\\pnidui.dll")),       // Network status icons
                PathBuf::from(format!("{system32}\\dsuiext.dll")),      // Directory service icons
                PathBuf::from(format!("{system32}\\ieframe.dll")),      // Internet Explorer icons
                PathBuf::from(format!("{system32}\\compstui.dll")),     // Printer icons
            ]
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
