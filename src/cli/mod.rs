use arbitrary::Arbitrary;
use clap::Parser;
use eyre::Result;
use std::ffi::OsString;
use to_args::ToArgs;

pub mod command;
pub mod global_args;
pub mod json_log_behaviour;
pub mod main;
pub mod to_args;
pub mod tracing;

use global_args::GlobalArgs;

#[derive(Parser, Debug, Arbitrary, PartialEq)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(flatten)]
    pub global_args: GlobalArgs,
    #[command(subcommand)]
    pub command: command::CliCommand,
}

impl ToArgs for Cli {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        args.extend(self.global_args.to_args());
        args.extend(self.command.to_args());
        args
    }
}

impl Cli {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}
