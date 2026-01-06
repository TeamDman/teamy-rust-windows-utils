use std::path::Path;
use std::path::PathBuf;

use crate::shell::pidl::Pidl;

pub trait PathExtensions {
    fn unc_canonicalize(&self) -> eyre::Result<PathBuf> {
        Ok(dunce::canonicalize(self.as_path())?)
    }
    fn unc_simplified(&self) -> &Path {
        dunce::simplified(self.as_path())
    }
    fn to_pidl(&self) -> eyre::Result<Pidl> {
        Pidl::try_new(self.as_path())
    }
    fn as_path(&self) -> &Path;
}

impl<T> PathExtensions for T
where
    T: AsRef<Path>,
{
    fn as_path(&self) -> &Path {
        self.as_ref()
    }
}
