use eyre::Context;
use eyre::Result;
use eyre::eyre;
use std::io::Write;
use std::sync::OnceLock;
use teamy_windows::console::console_attach;
use teamy_windows::console::console_create;
use teamy_windows::console::console_detach;
use teamy_windows::log::BufferSink;
use teamy_windows::tray::WM_TASKBAR_CREATED;
use teamy_windows::tray::WM_USER_TRAY_CALLBACK;
use teamy_windows::tray::delete_tray_icon;
use teamy_windows::tray::re_add_tray_icon;
use tracing::debug;
use tracing::error;
use tracing::info;
use windows::Win32::Foundation::HWND;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::POINT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::Console::ATTACH_PARENT_PROCESS;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;
use windows::core::w;

const CMD_SHOW_LOGS: usize = 0x1000;
const CMD_HIDE_LOGS: usize = 0x1001;
const CMD_EXIT_APP: usize = 0x1002;
const CMD_AHOY: usize = 0x1003;

#[derive(Clone)]
pub struct TrayConsoleConfig {
    pub inherited_console_available: bool,
    pub log_buffer: BufferSink,
}

static CONFIG: OnceLock<TrayConsoleConfig> = OnceLock::new();

pub fn configure_tray_console(config: TrayConsoleConfig) -> Result<()> {
    CONFIG
        .set(config)
        .map_err(|_| eyre!("Tray console configuration may only be set once"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConsoleMode {
    Detached,
    Inherited,
    Owned,
}

struct TrayConsoleState {
    mode: ConsoleMode,
    inherited_console_available: bool,
    log_buffer: BufferSink,
}

impl TrayConsoleState {
    fn new(config: TrayConsoleConfig) -> Self {
        let mode = if config.inherited_console_available {
            ConsoleMode::Inherited
        } else {
            ConsoleMode::Detached
        };
        Self {
            mode,
            inherited_console_available: config.inherited_console_available,
            log_buffer: config.log_buffer,
        }
    }

    fn can_show_logs(&self) -> bool {
        self.mode != ConsoleMode::Owned
    }

    fn can_hide_logs(&self) -> bool {
        self.mode == ConsoleMode::Owned
    }

    fn show_logs(&mut self) -> Result<()> {
        if !self.can_show_logs() {
            debug!("Show logs requested while already owning console");
            return Ok(());
        }

        if self.mode == ConsoleMode::Inherited {
            console_detach().wrap_err("Failed to detach from inherited console")?;
        }

        console_create().wrap_err("Failed to allocate dedicated console")?;
        self.replay_buffer()
            .wrap_err("Failed to replay buffered logs into new console")?;
        self.mode = ConsoleMode::Owned;
        info!("Console window allocated; new logs will stream live");
        Ok(())
    }

    fn hide_logs(&mut self) -> Result<()> {
        if !self.can_hide_logs() {
            debug!("Hide logs requested while console is already hidden");
            return Ok(());
        }

        console_detach().wrap_err("Failed to detach from dedicated console")?;
        if self.inherited_console_available {
            console_attach(ATTACH_PARENT_PROCESS)
                .wrap_err("Failed to reattach to parent console")?;
            self.mode = ConsoleMode::Inherited;
            info!("Logs routed back to parent console");
        } else {
            self.mode = ConsoleMode::Detached;
            info!("Logs hidden; no console attached");
        }
        Ok(())
    }

    fn replay_buffer(&self) -> Result<()> {
        let mut stdout = std::io::stdout();
        self.log_buffer
            .replay(&mut stdout)
            .wrap_err("Failed to write buffered logs to stdout")?;
        stdout.flush().ok();
        Ok(())
    }

    fn show_context_menu(&mut self, hwnd: HWND) {
        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }

        let menu = match unsafe { CreatePopupMenu() } {
            Ok(menu) => menu,
            Err(error) => {
                error!("Failed to create context menu: {error}");
                return;
            }
        };

        unsafe {
            if let Err(error) = AppendMenuW(menu, MF_STRING, CMD_SHOW_LOGS, w!("Show logs")) {
                error!("Failed to populate context menu: {error}");
            }
            if let Err(error) = AppendMenuW(menu, MF_STRING, CMD_HIDE_LOGS, w!("Hide logs")) {
                error!("Failed to populate context menu: {error}");
            }
            if let Err(error) = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null()) {
                error!("Failed to populate context menu: {error}");
            }
            if let Err(error) = AppendMenuW(menu, MF_STRING, CMD_AHOY, w!("Ahoy!")) {
                error!("Failed to populate context menu: {error}");
            }
            if let Err(error) = AppendMenuW(menu, MF_STRING, CMD_EXIT_APP, w!("Exit")) {
                error!("Failed to populate context menu: {error}");
            }

            if !self.can_show_logs() {
                let _ = EnableMenuItem(menu, CMD_SHOW_LOGS as u32, MF_BYCOMMAND | MF_GRAYED);
            }
            if !self.can_hide_logs() {
                let _ = EnableMenuItem(menu, CMD_HIDE_LOGS as u32, MF_BYCOMMAND | MF_GRAYED);
            }
        }

        let mut cursor_pos = POINT::default();
        unsafe { GetCursorPos(&mut cursor_pos) }.ok();
        let selection = unsafe {
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON | TPM_TOPALIGN | TPM_LEFTALIGN | TPM_RETURNCMD,
                cursor_pos.x,
                cursor_pos.y,
                None,
                hwnd,
                None,
            )
        }
        .0 as usize;

        if let Err(error) = unsafe { DestroyMenu(menu) } {
            error!("Failed to destroy context menu: {error}");
        }

        match selection {
            CMD_SHOW_LOGS => {
                if let Err(error) = self.show_logs() {
                    error!("Failed to show logs: {error}");
                }
            }
            CMD_HIDE_LOGS => {
                if let Err(error) = self.hide_logs() {
                    error!("Failed to hide logs: {error}");
                }
            }
            CMD_AHOY => {
                info!("Ahoy!");
            }
            CMD_EXIT_APP => unsafe {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            },
            _ => {}
        }
    }
}

fn store_state(hwnd: HWND, state: Box<TrayConsoleState>) {
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
    }
}

fn with_state<R>(hwnd: HWND, f: impl FnOnce(&mut TrayConsoleState) -> R) -> Option<R> {
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if raw == 0 {
        None
    } else {
        let state = unsafe { &mut *(raw as *mut TrayConsoleState) };
        Some(f(state))
    }
}

fn drop_state(hwnd: HWND) {
    let raw = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
    if raw != 0 {
        unsafe { drop(Box::from_raw(raw as *mut TrayConsoleState)) };
    }
}

/// Safety: This function is an extern "system" callback.
pub unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => match CONFIG.get().cloned() {
            Some(config) => {
                store_state(hwnd, Box::new(TrayConsoleState::new(config)));
                LRESULT(0)
            }
            None => {
                error!("Tray console configuration missing before window creation");
                LRESULT(-1)
            }
        },
        WM_USER_TRAY_CALLBACK => {
            match lparam.0 as u32 {
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    with_state(hwnd, |state| state.show_context_menu(hwnd));
                }
                WM_LBUTTONDBLCLK => {
                    with_state(hwnd, |state| {
                        if let Err(error) = state.show_logs() {
                            error!("Failed to show logs via double-click: {error}");
                        }
                    });
                }
                _ => {}
            }
            LRESULT(0)
        }
        m if m == *WM_TASKBAR_CREATED => {
            if let Err(error) = re_add_tray_icon() {
                error!("Failed to re-add tray icon after Explorer restart: {error}");
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) }.ok();
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Err(error) = delete_tray_icon(hwnd) {
                error!("Failed to delete tray icon: {error}");
            }
            drop_state(hwnd);
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
