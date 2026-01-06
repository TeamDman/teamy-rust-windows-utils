use crate::com::com_guard::ComGuard;
use crate::string::EasyPCWSTR;
use eyre::bail;
use std::path::Path;
use std::ptr;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::IShellFolder;
use windows::Win32::UI::Shell::SHBindToParent;
use windows::Win32::UI::Shell::SHOpenFolderAndSelectItems;
use windows::Win32::UI::Shell::SHParseDisplayName;

pub fn open_folder_and_select_items(path: impl AsRef<Path>) -> eyre::Result<()> {
    // Canonicalize path and normalize
    let path = path.as_ref().canonicalize()?;
    let path_str = path.to_string_lossy();
    let path_str = path_str.trim_start_matches(r"\\?\");

    // Ensure COM is initialized (some Shell calls rely on it)
    let _com_guard = ComGuard::new()?;

    unsafe {
        if path.is_dir() {
            // Open the folder itself
            let mut pidl_folder: *mut ITEMIDLIST = ptr::null_mut();
            SHParseDisplayName(
                path_str.easy_pcwstr()?.as_ref(),
                None,
                &mut pidl_folder,
                0,
                None,
            )?;
            if pidl_folder.is_null() {
                bail!("Failed to get PIDL for folder: {}", path.display());
            }

            SHOpenFolderAndSelectItems(pidl_folder as _, None, 0)?;
            CoTaskMemFree(Some(pidl_folder as _));
        } else {
            // For files: open parent folder and select the child PIDL
            let parent = path
                .parent()
                .ok_or_else(|| eyre::eyre!("Path has no parent: {}", path.display()))?;
            let parent_str = parent.to_string_lossy();
            let parent_str = parent_str.trim_start_matches(r"\\?\");

            let mut pidl_full: *mut ITEMIDLIST = ptr::null_mut();
            let mut child_pidl: *mut ITEMIDLIST = ptr::null_mut();
            let mut pidl_parent: *mut ITEMIDLIST = ptr::null_mut();

            SHParseDisplayName(
                path_str.easy_pcwstr()?.as_ref(),
                None,
                &mut pidl_full,
                0,
                None,
            )?;
            if pidl_full.is_null() {
                bail!("Failed to get PIDL for path: {}", path.display());
            }

            // Get a pointer to the child ID inside the full PIDL
            let _parent_folder: IShellFolder = SHBindToParent(pidl_full, Some(&mut child_pidl))?;

            SHParseDisplayName(
                parent_str.easy_pcwstr()?.as_ref(),
                None,
                &mut pidl_parent,
                0,
                None,
            )?;
            if pidl_parent.is_null() {
                bail!("Failed to get PIDL for parent: {}", parent.display());
            }

            let apidl = [child_pidl as *const ITEMIDLIST];
            SHOpenFolderAndSelectItems(pidl_parent as _, Some(&apidl), 0)?;

            CoTaskMemFree(Some(pidl_parent as _));
            CoTaskMemFree(Some(pidl_full as _));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() -> eyre::Result<()> {
        let path = file!();
        super::open_folder_and_select_items(path)?;
        Ok(())
    }
}
