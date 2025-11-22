use clap::{Args, Subcommand};
use eyre::Result;

pub mod list;

#[derive(Args, Debug)]
pub struct EntryArgs {
    #[command(subcommand)]
    pub command: EntryCommand,
}

impl EntryArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug)]
pub enum EntryCommand {
    List(list::EntryListArgs),
}

impl EntryCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            EntryCommand::List(args) => args.invoke(),
        }
    }
}
