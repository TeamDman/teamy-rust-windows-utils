use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod clipboard;
pub mod explorer;
pub mod icon;
pub mod mic;
pub mod window;

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum CliCommand {
    Clipboard(clipboard::ClipboardArgs),
    Explorer(explorer::ExplorerArgs),
    Icon(icon::IconArgs),
    Mic(mic::MicArgs),
    Window(window::WindowArgs),
}

impl ToArgs for CliCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            CliCommand::Clipboard(args) => {
                let mut ret = vec!["clipboard".into()];
                ret.extend(args.to_args());
                ret
            }
            CliCommand::Explorer(args) => {
                let mut ret = vec!["explorer".into()];
                ret.extend(args.to_args());
                ret
            }
            CliCommand::Icon(args) => {
                let mut ret = vec!["icon".into()];
                ret.extend(args.to_args());
                ret
            }
            CliCommand::Mic(args) => {
                let mut ret = vec!["mic".into()];
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
            CliCommand::Clipboard(args) => args.invoke(),
            CliCommand::Explorer(args) => args.invoke(),
            CliCommand::Icon(args) => args.invoke(),
            CliCommand::Mic(args) => args.invoke(),
            CliCommand::Window(args) => args.invoke(),
        }
    }
}
