use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod list;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct EntryArgs {
    #[command(subcommand)]
    pub command: EntryCommand,
}

impl ToArgs for EntryArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}

impl EntryArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum EntryCommand {
    List(list::EntryListArgs),
}

impl ToArgs for EntryCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            EntryCommand::List(args) => {
                let mut ret = vec!["list".into()];
                ret.extend(args.to_args());
                ret
            }
        }
    }
}

impl EntryCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            EntryCommand::List(args) => args.invoke(),
        }
    }
}
