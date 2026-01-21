use crate::cli::command::icon::browse::IconBrowseArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

/// Icon commands.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct IconArgs {
    #[command(subcommand)]
    pub command: IconCommand,
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum IconCommand {
    Browse(IconBrowseArgs),
}

impl IconArgs {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            IconCommand::Browse(args) => args.invoke(),
        }
    }
}

impl ToArgs for IconArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match &self.command {
            IconCommand::Browse(browse_args) => {
                args.push("browse".into());
                args.extend(browse_args.to_args());
            }
        }
        args
    }
}
