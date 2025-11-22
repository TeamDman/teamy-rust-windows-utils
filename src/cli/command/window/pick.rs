use crate::cli::to_args::ToArgs;
use crate::window::WindowInfo;
use crate::window::enumerate_windows;
use arbitrary::Arbitrary;
use clap::{Args, ValueEnum};
use cloud_terrastodon_user_input::Choice;
use cloud_terrastodon_user_input::PickerTui;
use eyre::Result;
use std::ffi::OsString;

#[derive(ValueEnum, Clone, Debug, PartialEq, Arbitrary)]
pub enum WindowPickArgsOutputFormat {
    Text,
    #[cfg(feature = "serde")]
    Json,
}

#[derive(Args, Debug, Arbitrary, PartialEq)]
pub struct WindowPickArgs {
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub many: bool,

    #[arg(long, short, default_value = "text")]
    pub output: WindowPickArgsOutputFormat,
}

impl ToArgs for WindowPickArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        if self.all {
            args.push("--all".into());
        }
        if self.many {
            args.push("--many".into());
        }
        if let Some(format) = self.output.to_possible_value() {
            args.push("--output".into());
            args.push(format.get_name().into());
        }
        args
    }
}

impl WindowPickArgs {
    pub fn invoke(self) -> Result<()> {
        let mut windows = enumerate_windows()?;

        if !self.all {
            windows.retain(|w| {
                let width = w.rect.right - w.rect.left;
                let height = w.rect.bottom - w.rect.top;
                w.is_visible && width > 0 && height > 0
            });
        }

        let picker: PickerTui<WindowInfo> =
            PickerTui::new(windows.into_iter().map(|window| Choice {
                key: format!("{} - {}", window.title, window.exe_path),
                value: window,
            }));


        if self.many {
            let selected_windows = picker.pick_many()?;
            match self.output {
                WindowPickArgsOutputFormat::Text => {
                    for window in selected_windows {
                        println!("{:?}\t{}\t{}", window.hwnd, window.title, window.exe_path);
                    }
                }
                #[cfg(feature = "serde")]
                WindowPickArgsOutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&selected_windows)?);
                }
            }
        } else {
            let selected_window = picker.pick_one()?;
            match self.output {
                WindowPickArgsOutputFormat::Text => {
                    println!(
                        "{:?}\t{}\t{}",
                        selected_window.hwnd, selected_window.title, selected_window.exe_path
                    );
                }
                #[cfg(feature = "serde")]
                WindowPickArgsOutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&selected_window)?);
                }
            }
        }

        Ok(())
    }
}
