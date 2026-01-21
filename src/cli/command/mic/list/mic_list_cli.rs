use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;

/// List microphones.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct MicListArgs;

impl MicListArgs {
    pub fn invoke(self) -> Result<()> {
        println!("Microphone list (stub)");
        Ok(())
    }
}

impl ToArgs for MicListArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
