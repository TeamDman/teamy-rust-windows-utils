use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::Subcommand;
use eyre::Result;
use std::ffi::OsString;

pub mod focus;
pub mod list;
pub mod pick;

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct WindowArgs {
    #[command(subcommand)]
    pub command: WindowCommand,
}

impl ToArgs for WindowArgs {
    fn to_args(&self) -> Vec<OsString> {
        self.command.to_args()
    }
}

impl WindowArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug, Arbitrary, PartialEq)]
pub enum WindowCommand {
    List(list::WindowListArgs),
    Focus(focus::WindowFocusArgs),
    Pick(pick::WindowPickArgs),
}

impl ToArgs for WindowCommand {
    fn to_args(&self) -> Vec<OsString> {
        match self {
            WindowCommand::List(args) => {
                let mut ret = vec!["list".into()];
                ret.extend(args.to_args());
                ret
            }
            WindowCommand::Focus(args) => {
                let mut ret = vec!["focus".into()];
                ret.extend(args.to_args());
                ret
            }
            WindowCommand::Pick(args) => {
                let mut ret = vec!["pick".into()];
                ret.extend(args.to_args());
                ret
            }
        }
    }
}

impl WindowCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            WindowCommand::List(args) => args.invoke(),
            WindowCommand::Focus(args) => args.invoke(),
            WindowCommand::Pick(args) => args.invoke(),
        }
    }
}
