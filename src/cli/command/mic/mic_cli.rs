use crate::cli::to_args::ToArgs;
use crate::cli::command::mic::list::MicListArgs;
use crate::cli::command::mic::record::MicRecordArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

/// Microphone commands.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct MicArgs {
    #[command(subcommand)]
    pub command: MicCommand,
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum MicCommand {
    List(MicListArgs),
    Record(MicRecordArgs),
}

impl MicArgs {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            MicCommand::List(args) => args.invoke(),
            MicCommand::Record(args) => args.invoke(),
        }
    }
}

impl ToArgs for MicArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match &self.command {
            MicCommand::List(list_args) => {
                args.push("list".into());
                args.extend(list_args.to_args());
            }
            MicCommand::Record(record_args) => {
                args.push("record".into());
                args.extend(record_args.to_args());
            }
        }
        args
    }
}
