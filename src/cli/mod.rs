use clap::Parser;
use eyre::Result;

pub mod command;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: command::CliCommand,
}

impl Cli {
    pub fn invoke(self) -> Result<()> {
        self.command.invoke()
    }
}
