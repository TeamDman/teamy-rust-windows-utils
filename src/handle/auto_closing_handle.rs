use std::ops::Deref;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Foundation::HANDLE;

/// Closes the contained handle when dropped.
pub struct AutoClosingHandle(HANDLE);

impl AutoClosingHandle {
    pub fn new(handle: HANDLE) -> Self {
        Self(handle)
    }
    pub fn into_inner(self) -> HANDLE {
        self.0
    }
}

impl Deref for AutoClosingHandle {
    type Target = HANDLE;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for AutoClosingHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}