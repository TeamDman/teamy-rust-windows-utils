use crate::window::enumerate_windows;
use clap::{Args, ValueEnum};
use eyre::Result;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum WindowListArgsOutputFormat {
    Text,
    #[cfg(feature = "serde")]
    Json,
}

#[derive(Args, Debug)]
pub struct WindowListArgs {
    #[arg(long)]
    pub all: bool,

    #[arg(long, short, default_value = "text")]
    pub output: WindowListArgsOutputFormat,
}

impl WindowListArgs {
    pub fn invoke(self) -> Result<()> {
        let mut windows = enumerate_windows()?;

        if !self.all {
            windows.retain(|w| {
                let width = w.rect.right - w.rect.left;
                let height = w.rect.bottom - w.rect.top;
                w.is_visible && width > 0 && height > 0
            });
        }

        #[cfg(feature = "serde")]
        if self.output == WindowListArgsOutputFormat::Json {
            let json = serde_json::to_string_pretty(&windows)?;
            println!("{}", json);
            return Ok(());
        }

        println!(
            "{:<10} {:<10} {:<10} {:<40} {:<20} Title",
            "HWND", "PID", "TID", "Class", "Rect"
        );
        println!(
            "{:-<10} {:-<10} {:-<10} {:-<40} {:-<20} {:-<20}",
            "", "", "", "", "", ""
        );

        for w in windows {
            let rect_str = format!(
                "{},{},{},{}",
                w.rect.left,
                w.rect.top,
                w.rect.right - w.rect.left,
                w.rect.bottom - w.rect.top
            );
            println!(
                "{:<10?} {:<10} {:<10} {:<40} {:<20} {}",
                w.hwnd, w.process_id, w.thread_id, w.class_name, rect_str, w.title
            );
        }

        Ok(())
    }
}
