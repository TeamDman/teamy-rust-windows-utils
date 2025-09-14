---
applyTo: '**/*.rs'
---

Here is the content of `HANDLE` from `windows-0.62.0/src/Windows/Win32/Foundation/mod.rs`:

```rust
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HANDLE(pub *mut core::ffi::c_void);
impl HANDLE {
    pub fn is_invalid(&self) -> bool {
        self.0 == -1 as _ || self.0 == 0 as _
    }
}
impl windows_core::Free for HANDLE {
    #[inline]
    unsafe fn free(&mut self) {
        if !self.is_invalid() {
            windows_link::link!("kernel32.dll" "system" fn CloseHandle(hobject : *mut core::ffi::c_void) -> i32);
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}
impl Default for HANDLE {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}
```

Here is the content of `windows-core-0.62.0/src/handles.rs`:

```rust
/// Custom code to free a handle.
///
/// This is similar to the [`Drop`] trait, and may be used to implement [`Drop`], but allows handles
/// to be dropped depending on context.
pub trait Free {
    /// Calls the handle's free function.
    ///
    /// # Safety
    /// The handle must be owned by the caller and safe to free.
    unsafe fn free(&mut self);
}

/// A wrapper to provide ownership for handles to automatically drop via the handle's [`Free`] trait.
#[repr(transparent)]
#[derive(PartialEq, Eq, Default, Debug)]
pub struct Owned<T: Free>(T);

impl<T: Free> Owned<T> {
    /// Takes ownership of the handle.
    ///
    /// # Safety
    ///
    /// The handle must be owned by the caller and safe to free.
    pub unsafe fn new(x: T) -> Self {
        Self(x)
    }
}

impl<T: Free> Drop for Owned<T> {
    fn drop(&mut self) {
        unsafe { self.0.free() };
    }
}

impl<T: Free> core::ops::Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Free> core::ops::DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
```

Therefore, we should ALWAYS prioritize wrapping `Handle` in `Owned<HANDLE>` so that we do not need to manually close handles, instead leveraging the `Drop` impl for `Owned<T>`.