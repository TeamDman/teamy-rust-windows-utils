#[cfg(feature = "cli")]
pub mod cli;
pub mod clipboard;
pub mod com;
pub mod console;
pub mod elevation;
pub mod event_loop;
pub mod handle;
pub mod hicon;
pub mod invocation;
pub mod job;
pub mod log;
pub mod module;
pub mod network;
pub mod shell;
pub mod storage;
pub mod string;
pub mod tray;
pub mod window;
pub mod audio;

// Re-export dunce for path normalization
pub use dunce;
