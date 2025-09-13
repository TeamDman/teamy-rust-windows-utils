//! Console related utilities
//! Attaching and detaching the console is useful when dealing with elevation, and when dealing with hiding the console window.
//! When elevating, I want the logs to go to the same console the user started the program from.
//! When creating system tray applications, I want the console to be hidden by default, but have it be restorable if the user uses a tray action to show logs.

mod ansi_support;
mod attach_to_console;
mod check_inheriting;
mod ctrl_handler;
mod hide;

pub use ansi_support::*;
pub use attach_to_console::*;
pub use check_inheriting::*;
pub use ctrl_handler::*;
pub use hide::*;
