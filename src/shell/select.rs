use crate::com::com_guard::ComGuard;
use crate::shell::path_extensions::PathExtensions;
use crate::shell::pidl::Pidl;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::SHOpenFolderAndSelectItems;

/// Opens Explorer windows and selects the specified items.
///
/// Items are grouped by their parent directory. For each unique parent directory,
/// an Explorer window is opened with those items selected.
///
/// See: <https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shopenfolderandselectitems>
pub fn open_folder_and_select_items<I, P>(paths: I) -> eyre::Result<()>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    // Ensure COM is initialized
    let _com_guard = ComGuard::new()?;

    // Group paths by parent directory
    // Both files AND directories are treated as items to select in their parent folder
    let mut grouped: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for path in paths {
        let path = path.as_ref().unc_canonicalize()?;

        let parent = path
            .parent()
            .ok_or_else(|| eyre::eyre!("Path has no parent: {}", path.display()))?
            .to_path_buf();
        grouped.entry(parent).or_default().push(path);
    }

    // Open each group
    for (parent_path, child_paths) in grouped {
        select_items_in_folder(&parent_path, &child_paths)?;
    }

    Ok(())
}

/// Internal helper: selects multiple items within a single parent folder.
fn select_items_in_folder(parent_path: &Path, child_paths: &[PathBuf]) -> eyre::Result<()> {
    // Get the parent folder's PIDL
    let pidl_parent = parent_path.to_pidl()?;

    // For each child, get its full PIDL and extract the relative child PIDL.
    // We must keep the full PIDLs alive because the child PIDLs point into their memory.
    let mut full_pidls: Vec<Pidl> = Vec::with_capacity(child_paths.len());

    for child_path in child_paths {
        full_pidls.push(child_path.to_pidl()?);
    }

    // Now that all owned PIDLs are stable in the vec, extract child pointers
    let apidl: Vec<*const ITEMIDLIST> = full_pidls
        .iter()
        .map(|pidl| pidl.child_pidl())
        .collect::<eyre::Result<Vec<_>>>()?
        .iter()
        .map(|p| p.as_ptr())
        .collect();

    unsafe {
        SHOpenFolderAndSelectItems(pidl_parent.as_ptr() as _, Some(&apidl), 0)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn single_file() -> eyre::Result<()> {
        let path = file!();
        open_folder_and_select_items([path])?;
        Ok(())
    }

    #[test]
    fn multiple_files_same_folder() -> eyre::Result<()> {
        // Select multiple files in the shell folder
        let paths = ["src/shell/select.rs", "src/shell/pidl.rs"];
        open_folder_and_select_items(paths)?;
        Ok(())
    }
}
