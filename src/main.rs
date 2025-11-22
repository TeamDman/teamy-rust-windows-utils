use clap::Parser;
use eyre::Result;
use teamy_windows::cli::Cli;
use windows::Win32::System::Com::COINIT_APARTMENTTHREADED;
use windows::Win32::System::Com::CoInitializeEx;

fn main() -> Result<()> {
    color_eyre::install()?;

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
    }

    let cli = Cli::parse();
    cli.invoke()?;

    Ok(())
}
