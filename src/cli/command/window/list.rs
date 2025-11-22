use crate::window::enumerate_windows;
use clap::Args;
use eyre::Result;

#[derive(Args, Debug)]
pub struct WindowListArgs {
    #[arg(long)]
    pub visible_only: bool,
}

impl WindowListArgs {
    pub fn invoke(self) -> Result<()> {
        let windows = enumerate_windows()?;

        println!(
            "{:<10} {:<10} {:<10} {:<40} {:<20} Title",
            "HWND", "PID", "TID", "Class", "Rect"
        );
        println!(
            "{:-<10} {:-<10} {:-<10} {:-<40} {:-<20} {:-<20}",
            "", "", "", "", "", ""
        );

        for w in windows {
            if self.visible_only && !w.is_visible {
                continue;
            }

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
