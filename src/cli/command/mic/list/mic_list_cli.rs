use crate::audio::TeamyImmDevice;
use crate::audio::list_audio_input_devices;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::owo_colors::colors::BrightBlack;
use color_eyre::owo_colors::colors::Yellow;
use color_eyre::owo_colors::colors::css::Gray;
use eyre::Result;
use facet::Facet;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::ops::Deref;

/// List microphones.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct MicListArgs;

impl MicListArgs {
    pub fn invoke(self) -> Result<()> {
        let devices = list_audio_input_devices()?;

        if std::io::stdout().is_terminal() {
            // emit coloured text
            if devices.is_empty() {
                println!("{}", "No microphones found.".red());
                return Ok(());
            }

            for device in devices {
                let default_marker = if device.is_default { " (default)" } else { "" };
                println!(
                    "({id}) {name} {default_marker}",
                    id = device.id.deref().fg::<BrightBlack>(),
                    name = device.name,
                    default_marker = default_marker.fg::<Yellow>()
                );
            }
        } else {
            // emit json
            structstruck::strike! {
                #[structstruck::each[derive(Facet)]]
                struct MicListOutput {
                    microphones: Vec<struct Mic {
                        id: String,
                        name: String,
                        is_default: bool,
                    }>,
                }
            }
            let output = MicListOutput {
                microphones: devices
                    .into_iter()
                    .map(|device| Mic {
                        id: device.id.0,
                        name: device.name,
                        is_default: device.is_default,
                    })
                    .collect(),
            };
            let json = facet_json::to_string(&output)?;
            println!("{}", json);
        }

        Ok(())
    }
}

impl ToArgs for MicListArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
