//! Application paths for home and cache directories.

mod app_home;
mod cache;

pub use app_home::*;
pub use cache::*;

pub const APP_HOME_ENV_VAR: &str = "TEAMY_WINDOWS_HOME_DIR";
pub const APP_HOME_DIR_NAME: &str = "teamy-windows";

pub const APP_CACHE_ENV_VAR: &str = "TEAMY_WINDOWS_CACHE_DIR";
pub const APP_CACHE_DIR_NAME: &str = "teamy-windows";
