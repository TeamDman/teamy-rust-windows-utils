pub mod window_proc;
use crate::window_proc::window_proc;
use teamy_rust_windows_utils::console::attach_ctrl_c_handler;
use teamy_rust_windows_utils::event_loop::run_message_loop;
use teamy_rust_windows_utils::hicon::application_icon::get_application_icon;
use teamy_rust_windows_utils::hicon::get_icon_from_current_module;
use teamy_rust_windows_utils::tray::add_tray_icon;
use teamy_rust_windows_utils::window::create_window_for_tray;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use windows::core::w;

pub fn main() -> eyre::Result<()> {
    let debug = true;

    SubscriberBuilder::default()
        .with_file(cfg!(debug_assertions))
        .with_line_number(cfg!(debug_assertions))
        .with_level(true)
        .with_target(false)
        .with_ansi(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        // .with_span_events(FmtSpan::NONE)
        // .with_timer(SystemTime)
        // .with_writer(writer)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder().parse_lossy(format!(
                "{default_log_level},{bevy_defaults}",
                default_log_level = match debug {
                    true => LevelFilter::DEBUG,
                    false => LevelFilter::INFO,
                },
                bevy_defaults = bevy_log::DEFAULT_FILTER
            ))
        }))
        .finish()
        .init();

    info!("Hello, world!");

    let window = create_window_for_tray(Some(window_proc))?;

    attach_ctrl_c_handler()?;

    let icon = get_icon_from_current_module(w!("aaa_my_icon")).or_else(|e1| {
        eprintln!("Failed to load embedded icon 'aaa_my_icon': {e1}");
        get_application_icon()
    })?;
    let tooltip = w!("Demo Tray");

    add_tray_icon(window, icon, tooltip)?;

    run_message_loop(Some(window))?;

    Ok(())
}
