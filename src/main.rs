fn main() -> eyre::Result<()> {
    #[cfg(feature = "cli")]
    {
        teamy_windows::cli::main::cli_main()?;
    }
    #[cfg(not(feature = "cli"))]
    {
        println!("CLI feature is disabled.");
    }

    Ok(())
}
