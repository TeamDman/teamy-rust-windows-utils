use crate::cli::to_args::ToArgs;
use crate::daemon::{DEFAULT_DAEMON_ID, open_window_via_daemon};
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct WindowOpenArgs {
    /// Logical daemon identifier. Use unique ids to target a specific daemon instance.
    #[arg(long = "daemon-id", default_value = DEFAULT_DAEMON_ID)]
    pub daemon_id: String,

    /// Window title.
    #[arg(long, default_value = "Teamy Windows Daemon Window")]
    pub title: String,
}

impl ToArgs for WindowOpenArgs {
    fn to_args(&self) -> Vec<OsString> {
        vec![
            "--daemon-id".into(),
            self.daemon_id.clone().into(),
            "--title".into(),
            self.title.clone().into(),
        ]
    }
}

impl WindowOpenArgs {
    pub fn invoke(self) -> Result<()> {
        let response = open_window_via_daemon(&self.daemon_id, &self.title)?;
        println!("{}", response.describe());
        Ok(())
    }
}
