fn main() -> eyre::Result<()> {
    #[cfg(feature = "cli")]
    {
        use clap::Parser;
        use teamy_windows::cli::Cli;

        color_eyre::install()?;

        let cli = Cli::parse();
        cli.invoke()?;
    }
    #[cfg(not(feature = "cli"))]
    {
        println!("CLI feature is disabled.");
    }

    Ok(())
}
