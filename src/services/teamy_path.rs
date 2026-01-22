//! Path type for efficient same-OS serialization.
//!
//! `TeamyPath` wraps a path and serializes using the raw OS string bytes,
//! avoiding UTF-8 lossy conversion for paths with special characters.

use std::convert::Infallible;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use facet::Facet;

/// A path that serializes efficiently for same-OS communication.
///
/// Uses `OsStr::as_encoded_bytes()` for serialization, preserving
/// the exact path representation without UTF-8 lossy conversion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
#[facet(proxy = Vec<u8>)]
pub struct TeamyPath(pub PathBuf);

impl TeamyPath {
    /// Create a new `TeamyPath` from a `PathBuf`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Get the inner `PathBuf`.
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl From<PathBuf> for TeamyPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl From<&Path> for TeamyPath {
    fn from(path: &Path) -> Self {
        Self(path.to_path_buf())
    }
}

impl From<String> for TeamyPath {
    fn from(s: String) -> Self {
        Self(PathBuf::from(s))
    }
}

impl From<&str> for TeamyPath {
    fn from(s: &str) -> Self {
        Self(PathBuf::from(s))
    }
}

impl From<OsString> for TeamyPath {
    fn from(s: OsString) -> Self {
        Self(PathBuf::from(s))
    }
}

impl From<&OsStr> for TeamyPath {
    fn from(s: &OsStr) -> Self {
        Self(PathBuf::from(s))
    }
}

impl From<TeamyPath> for PathBuf {
    fn from(path: TeamyPath) -> Self {
        path.0
    }
}

impl AsRef<Path> for TeamyPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl std::ops::Deref for TeamyPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
// Facet Proxy Conversions
// ============================================================================

/// Serialization: `&TeamyPath` -> `Vec<u8>` (OS-encoded bytes)
#[allow(clippy::infallible_try_from)]
impl TryFrom<&TeamyPath> for Vec<u8> {
    type Error = Infallible;
    fn try_from(path: &TeamyPath) -> Result<Self, Self::Error> {
        Ok(path.0.as_os_str().as_encoded_bytes().to_vec())
    }
}

/// Deserialization: `Vec<u8>` -> `TeamyPath`
#[allow(clippy::infallible_try_from)]
impl TryFrom<Vec<u8>> for TeamyPath {
    type Error = Infallible;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        // SAFETY: The bytes came from as_encoded_bytes() on the same OS
        let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(&bytes) };
        Ok(TeamyPath(PathBuf::from(os_str)))
    }
}
