use crate::console::{attach_ctrl_c_handler, console_detach, is_inheriting_console};

pub fn hide_default_console_or_attach_ctrl_handler() -> eyre::Result<()> {
    if is_inheriting_console() {
        attach_ctrl_c_handler()?;
    } else {
        _ = console_detach();
    };

    Ok(())
}