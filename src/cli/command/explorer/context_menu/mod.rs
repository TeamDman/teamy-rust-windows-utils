use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod entry;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct ContextMenuArgs {
    #[command(subcommand)]
    pub command: ContextMenuCommand,
}

impl ToArgs for ContextMenuArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}

impl ContextMenuArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum ContextMenuCommand {
    Entry(entry::EntryArgs),
}

impl ToArgs for ContextMenuCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            ContextMenuCommand::Entry(args) => {
                let mut ret = vec!["entry".into()];
                ret.extend(args.to_args());
                ret
            }
        }
    }
}

impl ContextMenuCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            ContextMenuCommand::Entry(args) => args.invoke(),
        }
    }
}
