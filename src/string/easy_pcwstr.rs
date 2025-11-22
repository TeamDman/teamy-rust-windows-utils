use crate::string::pcwstr_guard::PCWSTRGuard;
use clap::builder::OsStr;
use eyre::eyre;
use std::convert::Infallible;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use widestring::U16CString;

/// Conversion to `PCWSTRGuard` from various string types for easy FFI usage.
pub trait EasyPCWSTR {
    type Error;
    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error>;
}

impl EasyPCWSTR for U16CString {
    type Error = Infallible;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(self))
    }
}

impl EasyPCWSTR for &str {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_str(self).map_err(
            |_| eyre!("Failed to convert `&str` to U16CString: {}", self),
        )?))
    }
}

impl EasyPCWSTR for &OsString {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_os_str_truncate(&self)))
    }
}
impl EasyPCWSTR for &OsStr {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_os_str_truncate(&self)))
    }
}

impl EasyPCWSTR for &PathBuf {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_os_str_truncate(
            self.as_os_str(),
        )))
    }
}

impl EasyPCWSTR for &Path {
    type Error = eyre::Error;

    fn easy_pcwstr(self) -> eyre::Result<PCWSTRGuard, Self::Error> {
        Ok(PCWSTRGuard::new(U16CString::from_os_str_truncate(
            self.as_os_str(),
        )))
    }
}

#[cfg(test)]
mod test {
    use super::EasyPCWSTR;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;
    use widestring::U16CString;

    #[test]
    fn it_works() -> eyre::Result<()> {
        "Hello, World!".easy_pcwstr()?;
        OsString::from("asd").easy_pcwstr()?;
        "asd".to_string().easy_pcwstr()?;
        PathBuf::from("asd").easy_pcwstr()?;
        Path::new("asd").easy_pcwstr()?;
        U16CString::from_str("asd")?.easy_pcwstr()?;
        Ok(())
    }
}
