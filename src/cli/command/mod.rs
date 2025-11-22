use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod explorer;
pub mod window;

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum CliCommand {
    Explorer(explorer::ExplorerArgs),
    Window(window::WindowArgs),
}

impl ToArgs for CliCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            CliCommand::Explorer(args) => {
                let mut ret = vec!["explorer".into()];
                ret.extend(args.to_args());
                ret
            }
            CliCommand::Window(args) => {
                let mut ret = vec!["window".into()];
                ret.extend(args.to_args());
                ret
            }
        }
    }
}

impl CliCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            CliCommand::Explorer(args) => args.invoke(),
            CliCommand::Window(args) => args.invoke(),
        }
    }
}
