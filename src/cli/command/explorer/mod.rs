use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod context_menu;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct ExplorerArgs {
    #[command(subcommand)]
    pub command: ExplorerCommand,
}

impl ToArgs for ExplorerArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}

impl ExplorerArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum ExplorerCommand {
    ContextMenu(context_menu::ContextMenuArgs),
}

impl ToArgs for ExplorerCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            ExplorerCommand::ContextMenu(args) => {
                let mut ret = vec!["context-menu".into()];
                ret.extend(args.to_args());
                ret
            }
        }
    }
}

impl ExplorerCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            ExplorerCommand::ContextMenu(args) => args.invoke(),
        }
    }
}
