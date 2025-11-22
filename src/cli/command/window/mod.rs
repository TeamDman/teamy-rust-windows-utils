use clap::Args;
use clap::Subcommand;
use eyre::Result;

pub mod list;

#[derive(Args, Debug)]
pub struct WindowArgs {
    #[command(subcommand)]
    pub command: WindowCommand,
}

impl WindowArgs {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}

#[derive(Subcommand, Debug)]
pub enum WindowCommand {
    List(list::WindowListArgs),
}

impl WindowCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            WindowCommand::List(args) => args.invoke(),
        }
    }
}
