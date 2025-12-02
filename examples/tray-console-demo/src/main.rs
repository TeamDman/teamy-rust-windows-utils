pub mod window_proc;

use crate::window_proc::TrayConsoleConfig;
use crate::window_proc::configure_tray_console;
use crate::window_proc::window_proc;
use color_eyre::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use teamy_windows::console::hide_default_console_or_attach_ctrl_handler;
use teamy_windows::console::is_inheriting_console;
use teamy_windows::event_loop::run_message_loop;
use teamy_windows::hicon::application_icon::get_application_icon;
use teamy_windows::hicon::get_icon_from_current_module;
use teamy_windows::log::LOG_BUFFER;
use teamy_windows::tray::add_tray_icon;
use teamy_windows::window::create_window_for_tray;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::util::SubscriberInitExt;
use windows::core::w;

static HEARTBEAT_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn main() -> Result<()> {
    color_eyre::install()?;

    init_tracing();

    let started_with_inherited_console = is_inheriting_console();
    hide_default_console_or_attach_ctrl_handler()?;

    configure_tray_console(TrayConsoleConfig {
        inherited_console_available: started_with_inherited_console,
        log_buffer: LOG_BUFFER.clone(),
    })?;

    let window = create_window_for_tray(Some(window_proc))?;

    let icon = get_icon_from_current_module(w!("aaa_my_icon")).or_else(|e1| {
        eprintln!("Failed to load embedded icon 'aaa_my_icon': {e1}");
        get_application_icon()
    })?;
    let tooltip = w!("Tray Console Demo");

    add_tray_icon(window, icon, tooltip)?;

    info!("Tray console demo initialized");
    spawn_heartbeat_logger();

    run_message_loop(None)?;

    Ok(())
}

fn init_tracing() {
    let debug_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::builder().parse_lossy(format!(
            "{default_log_level},{bevy_defaults}",
            default_log_level = debug_level,
            bevy_defaults = bevy_log::DEFAULT_FILTER
        ))
    });

    let subscriber = SubscriberBuilder::default()
        .with_file(cfg!(debug_assertions))
        .with_line_number(cfg!(debug_assertions))
        .with_level(true)
        .with_target(false)
        .with_ansi(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr.and(LOG_BUFFER.clone()))
        .finish();

    if let Err(error) = subscriber.try_init() {
        eprintln!("Tracing already initialized? {error}");
    }
}

fn spawn_heartbeat_logger() {
    if HEARTBEAT_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    thread::spawn(|| {
        while HEARTBEAT_RUNNING.load(Ordering::SeqCst) {
            info!("Ahoy! Heartbeat log from tray-console-demo");
            thread::sleep(Duration::from_secs(1));
        }
    });
}

pub fn stop_heartbeat_logger() {
    HEARTBEAT_RUNNING.store(false, Ordering::SeqCst);
}
