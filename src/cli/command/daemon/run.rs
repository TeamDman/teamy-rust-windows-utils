use crate::cli::to_args::ToArgs;
use crate::daemon::{DEFAULT_DAEMON_ID, run_daemon};
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct DaemonRunArgs {
    /// Logical daemon identifier. Use unique ids to run multiple daemons in parallel.
    #[arg(long = "id", default_value = DEFAULT_DAEMON_ID)]
    pub daemon_id: String,
}

impl DaemonRunArgs {
    pub fn invoke(self) -> Result<()> {
        run_daemon(&self.daemon_id)
    }
}

impl ToArgs for DaemonRunArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec!["--id".into(), self.daemon_id.clone().into()]
    }
}
