use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod show;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct ClipboardArgs {
    #[command(subcommand)]
    pub command: ClipboardCommand,
}

impl ToArgs for ClipboardArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}

impl ClipboardArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum ClipboardCommand {
    Show(show::ClipboardShowArgs),
}

impl ToArgs for ClipboardCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            ClipboardCommand::Show(args) => {
                let mut ret = vec!["show".into()];
                ret.extend(args.to_args());
                ret
            }
        }
    }
}

impl ClipboardCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            ClipboardCommand::Show(args) => args.invoke(),
        }
    }
}
