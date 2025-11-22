use clap::Parser;
use teamy_windows::cli::Cli;
use eyre::Result;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

fn main() -> Result<()> {
    color_eyre::install()?;
    
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }

    let cli = Cli::parse();
    cli.invoke()?;

    Ok(())
}
