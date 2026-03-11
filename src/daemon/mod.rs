//! Proof-of-concept daemon lifecycle support.
//!
//! This is intentionally simple:
//! - each daemon instance gets a directory under the app cache dir
//! - the running daemon holds an exclusive lock file open for its lifetime
//! - metadata is written to a sidecar info file
//! - stop requests are delivered via a sentinel file
//!
//! This gives us a targeted, namespaced daemon identity before introducing a
//! richer IPC control plane.

use crate::paths::CACHE_DIR;
use crate::window::create_basic_window;
use eyre::{Context, Result, bail};
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use windows::Win32::System::Threading::DETACHED_PROCESS;

pub const DEFAULT_DAEMON_ID: &str = "default";
const DAEMON_CACHE_SUBDIR: &str = "daemon";
const LOCK_FILE_NAME: &str = "daemon.lock";
const INFO_FILE_NAME: &str = "daemon.info";
const STOP_FILE_NAME: &str = "daemon.stop";
const REQUESTS_DIR_NAME: &str = "requests";
const RESPONSES_DIR_NAME: &str = "responses";
const WINDOW_OPEN_PREFIX: &str = "window-open-";
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const WINDOW_OPEN_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const SHARING_VIOLATION: i32 = 32;
const LOCK_VIOLATION: i32 = 33;

#[derive(Clone, Debug)]
pub struct DaemonPaths {
    pub daemon_id: String,
    pub root_dir: PathBuf,
    pub instance_dir: PathBuf,
    pub lock_file: PathBuf,
    pub info_file: PathBuf,
    pub stop_file: PathBuf,
    pub requests_dir: PathBuf,
    pub responses_dir: PathBuf,
}

impl DaemonPaths {
    pub fn for_id(daemon_id: &str) -> Result<Self> {
        CACHE_DIR.ensure_dir()?;

        let daemon_id = normalize_daemon_id(daemon_id);
        let sanitized = sanitize_component(&daemon_id);
        let root_dir = CACHE_DIR.join(DAEMON_CACHE_SUBDIR);
        let instance_dir = root_dir.join(sanitized);
        let lock_file = instance_dir.join(LOCK_FILE_NAME);
        let info_file = instance_dir.join(INFO_FILE_NAME);
        let stop_file = instance_dir.join(STOP_FILE_NAME);
        let requests_dir = instance_dir.join(REQUESTS_DIR_NAME);
        let responses_dir = instance_dir.join(RESPONSES_DIR_NAME);

        Ok(Self {
            daemon_id,
            root_dir,
            instance_dir,
            lock_file,
            info_file,
            stop_file,
            requests_dir,
            responses_dir,
        })
    }

    pub fn ensure_instance_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.instance_dir)?;
        std::fs::create_dir_all(&self.requests_dir)?;
        std::fs::create_dir_all(&self.responses_dir)?;
        Ok(())
    }

    pub fn cleanup_stale_files(&self) -> Result<()> {
        remove_file_if_exists(&self.stop_file)?;
        remove_file_if_exists(&self.info_file)?;
        remove_file_if_exists(&self.lock_file)?;
        remove_children_if_dir_exists(&self.requests_dir)?;
        remove_children_if_dir_exists(&self.responses_dir)?;
        remove_dir_if_empty(&self.requests_dir)?;
        remove_dir_if_empty(&self.responses_dir)?;
        remove_dir_if_empty(&self.instance_dir)?;
        Ok(())
    }

    fn request_file(&self, request_id: &str) -> PathBuf {
        self.requests_dir
            .join(format!("{WINDOW_OPEN_PREFIX}{request_id}.request"))
    }

    fn request_file_processing(&self, request_id: &str) -> PathBuf {
        self.requests_dir
            .join(format!("{WINDOW_OPEN_PREFIX}{request_id}.processing"))
    }

    fn response_file(&self, request_id: &str) -> PathBuf {
        self.responses_dir
            .join(format!("{WINDOW_OPEN_PREFIX}{request_id}.response"))
    }
}

#[derive(Clone, Debug)]
pub struct DaemonInfo {
    pub daemon_id: String,
    pub pid: u32,
    pub started_unix_ms: u128,
}

impl DaemonInfo {
    pub fn new(daemon_id: String) -> Self {
        Self {
            daemon_id,
            pid: std::process::id(),
            started_unix_ms: now_unix_ms(),
        }
    }

    fn serialize(&self) -> String {
        format!(
            "daemon_id={}\npid={}\nstarted_unix_ms={}\n",
            self.daemon_id, self.pid, self.started_unix_ms
        )
    }

    fn deserialize(input: &str) -> Result<Self> {
        let mut daemon_id = None;
        let mut pid = None;
        let mut started_unix_ms = None;

        for line in input.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "daemon_id" => daemon_id = Some(value.trim().to_string()),
                "pid" => pid = Some(value.trim().parse().wrap_err("Invalid pid")?),
                "started_unix_ms" => {
                    started_unix_ms = Some(
                        value
                            .trim()
                            .parse()
                            .wrap_err("Invalid started_unix_ms")?,
                    )
                }
                _ => {}
            }
        }

        Ok(Self {
            daemon_id: daemon_id.ok_or_else(|| eyre::eyre!("Missing daemon_id"))?,
            pid: pid.ok_or_else(|| eyre::eyre!("Missing pid"))?,
            started_unix_ms: started_unix_ms
                .ok_or_else(|| eyre::eyre!("Missing started_unix_ms"))?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct DaemonStatus {
    pub paths: DaemonPaths,
    pub info: Option<DaemonInfo>,
    pub is_running: bool,
}

#[derive(Clone, Debug)]
struct WindowOpenRequest {
    request_id: String,
    title: String,
}

impl WindowOpenRequest {
    fn new(title: String) -> Self {
        Self {
            request_id: format!("{}-{}", now_unix_ms(), std::process::id()),
            title,
        }
    }

    fn serialize(&self) -> String {
        format!("request_id={}\ntitle={}\n", self.request_id, self.title)
    }

    fn deserialize(input: &str) -> Result<Self> {
        let mut request_id = None;
        let mut title = None;

        for line in input.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "request_id" => request_id = Some(value.trim().to_string()),
                "title" => title = Some(value.to_string()),
                _ => {}
            }
        }

        Ok(Self {
            request_id: request_id.ok_or_else(|| eyre::eyre!("Missing request_id"))?,
            title: title.ok_or_else(|| eyre::eyre!("Missing title"))?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WindowOpenResponse {
    pub daemon_id: String,
    pub window_id: String,
    pub hwnd: isize,
}

impl WindowOpenResponse {
    fn serialize(&self) -> String {
        format!(
            "daemon_id={}\nwindow_id={}\nhwnd={}\n",
            self.daemon_id, self.window_id, self.hwnd
        )
    }

    fn deserialize(input: &str) -> Result<Self> {
        let mut daemon_id = None;
        let mut window_id = None;
        let mut hwnd = None;

        for line in input.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "daemon_id" => daemon_id = Some(value.trim().to_string()),
                "window_id" => window_id = Some(value.trim().to_string()),
                "hwnd" => hwnd = Some(value.trim().parse().wrap_err("Invalid hwnd")?),
                _ => {}
            }
        }

        Ok(Self {
            daemon_id: daemon_id.ok_or_else(|| eyre::eyre!("Missing daemon_id"))?,
            window_id: window_id.ok_or_else(|| eyre::eyre!("Missing window_id"))?,
            hwnd: hwnd.ok_or_else(|| eyre::eyre!("Missing hwnd"))?,
        })
    }

    pub fn describe(&self) -> String {
        format!(
            "daemon_id={}\nwindow_id={}\nhwnd={}",
            self.daemon_id, self.window_id, self.hwnd
        )
    }
}

impl DaemonStatus {
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("daemon_id={}", self.paths.daemon_id));
        lines.push(format!("running={}", self.is_running));
        lines.push(format!("instance_dir={}", self.paths.instance_dir.display()));
        if let Some(info) = &self.info {
            lines.push(format!("pid={}", info.pid));
            lines.push(format!("started_unix_ms={}", info.started_unix_ms));
        }
        lines.join("\n")
    }
}

pub fn daemon_status(daemon_id: &str) -> Result<DaemonStatus> {
    let paths = DaemonPaths::for_id(daemon_id)?;
    let info = read_info(&paths)?;
    let is_running = is_lock_held_by_running_daemon(&paths)?;

    Ok(DaemonStatus {
        paths,
        info,
        is_running,
    })
}

pub fn start_daemon(daemon_id: &str) -> Result<DaemonStatus> {
    let status = daemon_status(daemon_id)?;
    if status.is_running {
        return Ok(status);
    }

    status.paths.ensure_instance_dir()?;
    status.paths.cleanup_stale_files()?;

    let mut child = spawn_detached_daemon_process(&status.paths.daemon_id)?;
    debug!(
        daemon_id = %status.paths.daemon_id,
        child_id = ?child.id(),
        "Spawned detached daemon process"
    );

    let start = Instant::now();
    loop {
        let status = daemon_status(daemon_id)?;
        if status.is_running {
            return Ok(status);
        }
        if start.elapsed() > READY_TIMEOUT {
            let _ = child.try_wait();
            bail!("Timed out waiting for daemon {} to become ready", daemon_id);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

pub fn request_daemon_stop(daemon_id: &str) -> Result<DaemonStatus> {
    let status = daemon_status(daemon_id)?;
    if !status.is_running {
        return Ok(status);
    }

    status.paths.ensure_instance_dir()?;
    std::fs::write(&status.paths.stop_file, b"stop\n")
        .wrap_err("Failed to write daemon stop sentinel")?;

    let start = Instant::now();
    loop {
        let current = daemon_status(daemon_id)?;
        if !current.is_running {
            current.paths.cleanup_stale_files()?;
            return daemon_status(daemon_id);
        }
        if start.elapsed() > SHUTDOWN_TIMEOUT {
            bail!("Timed out waiting for daemon {} to stop", daemon_id);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

pub fn open_window_via_daemon(daemon_id: &str, title: &str) -> Result<WindowOpenResponse> {
    let status = start_daemon(daemon_id)?;
    let request = WindowOpenRequest::new(title.to_string());
    let request_file = status.paths.request_file(&request.request_id);
    let response_file = status.paths.response_file(&request.request_id);
    let temp_request_file = request_file.with_extension("request.tmp");

    remove_file_if_exists(&response_file)?;
    std::fs::write(&temp_request_file, request.serialize())
        .wrap_err("Failed to write window-open request")?;
    std::fs::rename(&temp_request_file, &request_file)
        .wrap_err("Failed to publish window-open request")?;

    let start = Instant::now();
    loop {
        if response_file.exists() {
            let response = read_window_open_response(&response_file)?;
            remove_file_if_exists(&response_file)?;
            return Ok(response);
        }
        if start.elapsed() > WINDOW_OPEN_TIMEOUT {
            bail!(
                "Timed out waiting for daemon {} to open a window",
                status.paths.daemon_id
            );
        }
        thread::sleep(POLL_INTERVAL);
    }
}

pub fn run_daemon(daemon_id: &str) -> Result<()> {
    let paths = DaemonPaths::for_id(daemon_id)?;
    paths.ensure_instance_dir()?;
    remove_file_if_exists(&paths.stop_file)?;

    let lock = acquire_daemon_lock(&paths)?;
    let info = DaemonInfo::new(paths.daemon_id.clone());
    write_info(&paths, &info)?;

    println!("{}", info.daemon_id);
    std::io::stdout().flush()?;

    info!(
        daemon_id = %info.daemon_id,
        pid = info.pid,
        instance_dir = %paths.instance_dir.display(),
        "Daemon started"
    );

    loop {
        if paths.stop_file.exists() {
            info!(daemon_id = %info.daemon_id, "Daemon stop requested");
            break;
        }
        handle_window_open_requests(&paths)?;
        thread::sleep(Duration::from_millis(250));
    }

    remove_file_if_exists(&paths.stop_file)?;
    remove_file_if_exists(&paths.info_file)?;
    drop(lock);
    remove_file_if_exists(&paths.lock_file)?;
    remove_children_if_dir_exists(&paths.requests_dir)?;
    remove_children_if_dir_exists(&paths.responses_dir)?;
    remove_dir_if_empty(&paths.requests_dir)?;
    remove_dir_if_empty(&paths.responses_dir)?;
    remove_dir_if_empty(&paths.instance_dir)?;

    Ok(())
}

fn spawn_detached_daemon_process(daemon_id: &str) -> Result<Child> {
    let current_exe = std::env::current_exe().wrap_err("Failed to resolve current executable")?;
    let mut command = Command::new(current_exe);
    command
        .arg("daemon")
        .arg("run")
        .arg("--id")
        .arg(daemon_id)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS.0);
    let child = command.spawn().wrap_err("Failed to spawn detached daemon")?;
    Ok(child)
}

fn acquire_daemon_lock(paths: &DaemonPaths) -> Result<File> {
    paths.ensure_instance_dir()?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&paths.lock_file)
        .wrap_err_with(|| format!("Failed to acquire daemon lock at {}", paths.lock_file.display()))?;
    Ok(file)
}

fn is_lock_held_by_running_daemon(paths: &DaemonPaths) -> Result<bool> {
    if !paths.lock_file.exists() {
        return Ok(false);
    }

    match OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&paths.lock_file)
    {
        Ok(file) => {
            drop(file);
            Ok(false)
        }
        Err(err) if is_windows_lock_contention(&err) => Ok(true),
        Err(err) if err.kind() == ErrorKind::PermissionDenied => Ok(true),
        Err(err) => Err(err).wrap_err_with(|| {
            format!(
                "Failed to inspect daemon lock state at {}",
                paths.lock_file.display()
            )
        }),
    }
}

fn is_windows_lock_contention(err: &std::io::Error) -> bool {
    matches!(err.raw_os_error(), Some(SHARING_VIOLATION | LOCK_VIOLATION))
}

fn write_info(paths: &DaemonPaths, info: &DaemonInfo) -> Result<()> {
    std::fs::write(&paths.info_file, info.serialize())
        .wrap_err_with(|| format!("Failed to write daemon info at {}", paths.info_file.display()))
}

fn read_window_open_response(path: &Path) -> Result<WindowOpenResponse> {
    let mut contents = String::new();
    File::open(path)
        .wrap_err_with(|| format!("Failed to open window response at {}", path.display()))?
        .read_to_string(&mut contents)
        .wrap_err("Failed to read window response")?;

    WindowOpenResponse::deserialize(&contents)
}

fn handle_window_open_requests(paths: &DaemonPaths) -> Result<()> {
    if !paths.requests_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&paths.requests_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("request") {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(WINDOW_OPEN_PREFIX) {
            continue;
        }
        let Some(request_id) = file_name
            .strip_prefix(WINDOW_OPEN_PREFIX)
            .and_then(|value| value.strip_suffix(".request"))
        else {
            continue;
        };

        let processing_path = paths.request_file_processing(request_id);
        if let Err(err) = std::fs::rename(&path, &processing_path) {
            if err.kind() == ErrorKind::NotFound {
                continue;
            }
            return Err(err).wrap_err_with(|| {
                format!("Failed to claim request file {}", path.display())
            });
        }

        let request = match read_window_open_request(&processing_path) {
            Ok(request) => request,
            Err(err) => {
                error!(error = %err, path = %processing_path.display(), "Failed to parse window-open request");
                remove_file_if_exists(&processing_path)?;
                continue;
            }
        };

        let response_path = paths.response_file(&request.request_id);
        let daemon_id = paths.daemon_id.clone();
        thread::spawn(move || {
            if let Err(err) = spawn_window_for_request(daemon_id, request, response_path) {
                error!(error = %err, "Failed to spawn requested window");
            }
        });

        remove_file_if_exists(&processing_path)?;
    }

    Ok(())
}

fn read_window_open_request(path: &Path) -> Result<WindowOpenRequest> {
    let mut contents = String::new();
    File::open(path)
        .wrap_err_with(|| format!("Failed to open window request at {}", path.display()))?
        .read_to_string(&mut contents)
        .wrap_err("Failed to read window request")?;

    WindowOpenRequest::deserialize(&contents)
}

fn spawn_window_for_request(
    daemon_id: String,
    request: WindowOpenRequest,
    response_path: PathBuf,
) -> Result<()> {
    let hwnd = create_basic_window(&request.title)?;

    let response = WindowOpenResponse {
        daemon_id,
        window_id: request.request_id,
        hwnd: hwnd.0 as isize,
    };
    std::fs::write(&response_path, response.serialize()).wrap_err_with(|| {
        format!(
            "Failed to write window response at {}",
            response_path.display()
        )
    })?;

    info!(
        window_id = %response.window_id,
        hwnd = response.hwnd,
        title = %request.title,
        "Opened daemon-backed window"
    );

    crate::event_loop::run_message_loop(Some(hwnd))?;
    Ok(())
}

fn read_info(paths: &DaemonPaths) -> Result<Option<DaemonInfo>> {
    if !paths.info_file.exists() {
        return Ok(None);
    }

    let mut contents = String::new();
    File::open(&paths.info_file)
        .wrap_err_with(|| format!("Failed to open daemon info at {}", paths.info_file.display()))?
        .read_to_string(&mut contents)
        .wrap_err("Failed to read daemon info")?;

    Ok(Some(DaemonInfo::deserialize(&contents)?))
}

fn remove_file_if_exists(path: &PathBuf) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).wrap_err_with(|| format!("Failed to remove {}", path.display())),
    }
}

fn remove_children_if_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let child_path = entry.path();
        if child_path.is_file() {
            if let Err(err) = std::fs::remove_file(&child_path) {
                if err.kind() != ErrorKind::NotFound {
                    warn!(path = %child_path.display(), error = %err, "Failed to remove daemon child file during cleanup");
                }
            }
        }
    }

    Ok(())
}

fn remove_dir_if_empty(path: &PathBuf) -> Result<()> {
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) if err.kind() == ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(err) => Err(err).wrap_err_with(|| format!("Failed to remove {}", path.display())),
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn normalize_daemon_id(daemon_id: &str) -> String {
    let trimmed = daemon_id.trim();
    if trimmed.is_empty() {
        DEFAULT_DAEMON_ID.to_string()
    } else {
        trimmed.to_string()
    }
}

fn sanitize_component(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            output.push(ch);
        } else {
            output.push('_');
        }
    }

    if output.is_empty() {
        DEFAULT_DAEMON_ID.to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_component_replaces_path_separators() {
        assert_eq!(sanitize_component("test/worker:1"), "test_worker_1");
    }

    #[test]
    fn daemon_info_round_trips() {
        let info = DaemonInfo {
            daemon_id: "abc".to_string(),
            pid: 42,
            started_unix_ms: 123,
        };
        let round_trip = DaemonInfo::deserialize(&info.serialize()).unwrap();
        assert_eq!(round_trip.daemon_id, info.daemon_id);
        assert_eq!(round_trip.pid, info.pid);
        assert_eq!(round_trip.started_unix_ms, info.started_unix_ms);
    }
}
