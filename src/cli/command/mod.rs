use clap::Subcommand;
use eyre::Result;

pub mod explorer;
pub mod window;

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    Explorer(explorer::ExplorerArgs),
    Window(window::WindowArgs),
}

impl CliCommand {
    pub fn invoke(self) -> Result<()> {
        match self {
            CliCommand::Explorer(args) => args.invoke(),
            CliCommand::Window(args) => args.invoke(),
        }
    }
}
