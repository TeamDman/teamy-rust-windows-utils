use clap::Subcommand;
use eyre::Result;

pub mod explorer;

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    Explorer(explorer::ExplorerArgs),
}

impl CliCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            CliCommand::Explorer(args) => args.invoke(),
        }
    }
}
