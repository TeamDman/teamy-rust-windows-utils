use crate::cli::Cli;
use crate::cli::tracing::init_tracing;
use clap::Parser;

pub fn cli_main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    init_tracing(
        cli.global_args.log_level(),
        cli.global_args.json_log_behaviour(),
    )?;
    cli.invoke()?;
    Ok(())
}
