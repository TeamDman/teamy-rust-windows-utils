use crate::audio::list_audio_input_devices;
use crate::cli::to_args::ToArgs;
use arbitrary::Arbitrary;
use clap::Args;
use clap::ValueEnum;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::owo_colors::colors::BrightBlack;
use color_eyre::owo_colors::colors::Yellow;
use eyre::Result;
use facet::Facet;
use facet_pretty::ColorMode;
use facet_pretty::PrettyPrinter;
use std::ffi::OsString;
use std::io::IsTerminal;
use std::ops::Deref;

/// List microphones.
#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct MicListArgs {
    /// Output format.
    #[clap(long, value_enum, default_value_t = OutputFormat::Auto)]
    pub output_format: OutputFormat,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, Hash, Arbitrary)]
pub enum OutputFormat {
    Auto,
    Text,
    Facet,
    Json,
}
impl MicListArgs {
    pub fn invoke(mut self) -> Result<()> {
        let is_terminal = std::io::stdout().is_terminal();
        if matches!(self.output_format, OutputFormat::Auto) {
            self.output_format = if is_terminal {
                OutputFormat::Text
            } else {
                OutputFormat::Json
            };
        }

        let devices = list_audio_input_devices()?;

        match self.output_format {
            OutputFormat::Auto => unreachable!(),
            OutputFormat::Text => {
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
            }
            OutputFormat::Json | OutputFormat::Facet => {
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
                let mics: Vec<Mic> = devices
                    .into_iter()
                    .map(|device| Mic {
                        id: device.id.0,
                        name: device.name,
                        is_default: device.is_default,
                    })
                    .collect();
                match (is_terminal, &self.output_format) {
                    (true, OutputFormat::Facet) => {
                        let output = MicListOutput { microphones: mics };
                        let out = PrettyPrinter::new()
                            .with_colors(ColorMode::Always)
                            .with_doc_comments(true)
                            .format(&output);
                        println!("{}", out);
                    }
                    (false, OutputFormat::Facet) => {
                        let output = MicListOutput { microphones: mics };
                        let out = PrettyPrinter::new()
                            .with_colors(ColorMode::Never)
                            .format(&output);
                        println!("{}", out);
                    }
                    (true, OutputFormat::Json) => {
                        // Output array directly for easier PowerShell piping
                        let json = facet_json::to_string_pretty(&mics)?;
                        println!("{}", json);
                    }
                    (false, OutputFormat::Json) => {
                        // Output array directly for easier PowerShell piping
                        let json = facet_json::to_string(&mics)?;
                        println!("{}", json);
                    }
                    _ => unreachable!(),
                }
            }
        }

        Ok(())
    }
}

impl ToArgs for MicListArgs {
    fn to_args(&self) -> Vec<OsString> {
        Vec::new()
    }
}
