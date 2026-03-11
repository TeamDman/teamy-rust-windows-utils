use super::run::DaemonRunArgs;
use super::start::DaemonStartArgs;
use super::status::DaemonStatusArgs;
use super::stop::DaemonStopArgs;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

/// Daemon proof-of-concept commands.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum DaemonCommand {
    /// Start a daemon instance.
    Start(DaemonStartArgs),
    /// Show daemon instance status.
    Status(DaemonStatusArgs),
    /// Request that a daemon instance stop.
    Stop(DaemonStopArgs),
    /// Run a daemon instance in the current process.
    Run(DaemonRunArgs),
}

impl DaemonArgs {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            DaemonCommand::Start(args) => args.invoke(),
            DaemonCommand::Status(args) => args.invoke(),
            DaemonCommand::Stop(args) => args.invoke(),
            DaemonCommand::Run(args) => args.invoke(),
        }
    }
}

impl ToArgs for DaemonArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        match &self.command {
            DaemonCommand::Start(start_args) => {
                args.push("start".into());
                args.extend(start_args.to_args());
            }
            DaemonCommand::Status(status_args) => {
                args.push("status".into());
                args.extend(status_args.to_args());
            }
            DaemonCommand::Stop(stop_args) => {
                args.push("stop".into());
                args.extend(stop_args.to_args());
            }
            DaemonCommand::Run(run_args) => {
                args.push("run".into());
                args.extend(run_args.to_args());
            }
        }
        args
    }
}
