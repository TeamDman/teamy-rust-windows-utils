use clap::{Args, Subcommand};
use eyre::Result;

pub mod context_menu;

#[derive(Args, Debug)]
pub struct ExplorerArgs {
    #[command(subcommand)]
    pub command: ExplorerCommand,
}

impl ExplorerArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug)]
pub enum ExplorerCommand {
    ContextMenu(context_menu::ContextMenuArgs),
}

impl ExplorerCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            ExplorerCommand::ContextMenu(args) => args.invoke(),
        }
    }
}
