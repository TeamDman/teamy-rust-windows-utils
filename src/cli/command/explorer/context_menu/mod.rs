use clap::{Args, Subcommand};
use eyre::Result;

pub mod entry;

#[derive(Args, Debug)]
pub struct ContextMenuArgs {
    #[command(subcommand)]
    pub command: ContextMenuCommand,
}

impl ContextMenuArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug)]
pub enum ContextMenuCommand {
    Entry(entry::EntryArgs),
}

impl ContextMenuCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            ContextMenuCommand::Entry(args) => args.invoke(),
        }
    }
}
