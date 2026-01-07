use crate::cli::to_args::ToArgs;
use crate::shell::select::open_folder_and_select_items;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;
use std::path::PathBuf;

/// Opens Explorer and selects the specified path(s).
///
/// Multiple paths in the same parent folder will be selected together.
/// Paths in different folders will open separate Explorer windows.
#[derive(Args, Debug, PartialEq)]
pub struct ShowArgs {
    /// The path(s) to show in Explorer
    #[arg(required = true)]
    pub paths: Vec<PathBuf>,
}

impl<'a> Arbitrary<'a> for ShowArgs {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut paths = Vec::<PathBuf>::arbitrary(u)?;
        // Ensure at least one path and no empty paths
        if paths.is_empty() {
            paths.push(PathBuf::from("."));
        }
        paths.retain(|p| !p.as_os_str().is_empty());
        if paths.is_empty() {
            paths.push(PathBuf::from("."));
        }
        Ok(ShowArgs { paths })
    }
}

impl ToArgs for ShowArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.paths.iter().map(|p| p.clone().into()).collect()
    }
}

impl ShowArgs {
    pub fn invoke(self) -> Result<()> {
        open_folder_and_select_items(&self.paths)?;
        Ok(())
    }
}
