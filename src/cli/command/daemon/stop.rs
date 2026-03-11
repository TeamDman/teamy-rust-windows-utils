use crate::cli::to_args::ToArgs;
use crate::daemon::{DEFAULT_DAEMON_ID, request_daemon_stop};
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct DaemonStopArgs {
    /// Logical daemon identifier. Use unique ids to target a specific daemon instance.
    #[arg(long = "id", default_value = DEFAULT_DAEMON_ID)]
    pub daemon_id: String,
}

impl DaemonStopArgs {
    pub fn invoke(self) -> Result<()> {
        let status = request_daemon_stop(&self.daemon_id)?;
        println!("{}", status.describe());
        Ok(())
    }
}

impl ToArgs for DaemonStopArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec!["--id".into(), self.daemon_id.clone().into()]
    }
}
