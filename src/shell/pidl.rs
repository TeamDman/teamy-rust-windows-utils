use crate::shell::path_extensions::PathExtensions;
use crate::string::EasyPCWSTR;
use std::path::Path;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::IShellFolder;
use windows::Win32::UI::Shell::SHBindToParent;
use windows::Win32::UI::Shell::SHParseDisplayName;

/// RAII wrapper for an owned PIDL (pointer to ITEMIDLIST) that automatically frees with CoTaskMemFree.
pub struct Pidl(pub *mut ITEMIDLIST);

impl Pidl {
    pub fn try_new(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        unsafe {
            SHParseDisplayName(
                path.as_ref().unc_simplified().easy_pcwstr()?.as_ref(),
                None,
                &mut pidl,
                0,
                None,
            )?;
        }
        Ok(Self(pidl))
    }

    /// Takes ownership of a raw PIDL pointer.
    ///
    /// # Safety
    /// The pointer must be a valid PIDL allocated by the shell (e.g., via `SHParseDisplayName`)
    /// and must be owned (i.e., caller is responsible for freeing it).
    pub unsafe fn from_raw(ptr: *mut ITEMIDLIST) -> Self {
        Self(ptr)
    }

    /// Returns the raw pointer (does not transfer ownership).
    pub fn as_ptr(&self) -> *mut ITEMIDLIST {
        self.0
    }

    /// Returns a borrowed view of this PIDL.
    pub fn as_borrowed(&self) -> BorrowedPidl<'_> {
        BorrowedPidl {
            ptr: self.0 as *const ITEMIDLIST,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Extracts the relative child PIDL from this full PIDL.
    ///
    /// Calls `SHBindToParent` to get a pointer to the last component of the PIDL,
    /// which represents the item relative to its parent folder.
    ///
    /// # Safety
    /// The returned `BorrowedPidl` points into `self`'s memory. The caller must ensure
    /// that `self` outlives the returned `BorrowedPidl`.
    pub fn child_pidl(&self) -> eyre::Result<BorrowedPidl<'_>> {
        let mut child_pidl_raw: *mut ITEMIDLIST = std::ptr::null_mut();
        let _parent_folder: IShellFolder =
            unsafe { SHBindToParent(self.0, Some(&mut child_pidl_raw))? };
        Ok(unsafe { BorrowedPidl::from_raw(child_pidl_raw as *const _) })
    }
}

impl Drop for Pidl {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CoTaskMemFree(Some(self.0 as _));
            }
        }
    }
}

/// A borrowed PIDL that does not own its memory.
///
/// This is used for child PIDLs returned by `SHBindToParent`, which point into
/// the memory of the parent PIDL and must not be freed separately.
#[derive(Clone, Copy)]
pub struct BorrowedPidl<'a> {
    ptr: *const ITEMIDLIST,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> BorrowedPidl<'a> {
    /// Creates a borrowed PIDL from a raw pointer.
    ///
    /// # Safety
    /// The pointer must be valid for the lifetime `'a` and must point to valid PIDL data.
    pub unsafe fn from_raw(ptr: *const ITEMIDLIST) -> Self {
        Self {
            ptr,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Returns the raw pointer.
    pub fn as_ptr(&self) -> *const ITEMIDLIST {
        self.ptr
    }
}
