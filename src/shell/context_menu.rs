use crate::com::com_guard::ComGuard;
use crate::shell::path_extensions::PathExtensions;
use crate::string::EasyPCWSTR;
use eyre::Result;
use eyre::bail;
use std::path::Path;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Shell::Common::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

#[derive(Debug, Clone)]
pub struct ContextMenuEntry {
    pub id: u32,
    pub label: String,
    pub verb: String,
    pub sub_items: Vec<ContextMenuEntry>,
    pub is_separator: bool,
}

/// # Safety
///
/// This function calls unsafe Windows APIs.
pub unsafe fn get_context_menu_entries(path: impl AsRef<Path>) -> Result<Vec<ContextMenuEntry>> {
    // Canonicalize path, SHParseDisplayName doesn't always like the verbatim prefix \\?\
    let path = path.as_ref().unc_canonicalize()?;

    // 1. Initialize COM (Required for Shell Extensions)
    // We use a guard to ensure we uninitialize if we were the ones (or the refcount) that initialized it.
    let _com_guard = ComGuard::new()?;

    // 2. Convert Path to PIDL (Pointer to Item ID List)
    // SHParseDisplayName is the modern way to get a PIDL from a path
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
    let mut sfgao_out = 0;

    // Note: This expects a full absolute path
    // We ensure the path is absolute before calling this.
    unsafe {
        SHParseDisplayName(
            path.easy_pcwstr()?.as_ref(),
            None,
            &mut pidl,
            0,
            Some(&mut sfgao_out),
        )
    }?;

    if pidl.is_null() {
        bail!("Failed to get PIDL for path: {}", path.display());
    }

    // 3. Bind to the Parent Folder
    // We need the IShellFolder of the parent, and the relative PIDL of the child
    let mut child_pidl: *mut ITEMIDLIST = std::ptr::null_mut();

    let parent_folder: IShellFolder = unsafe { SHBindToParent(pidl, Some(&mut child_pidl)) }?;

    // 4. Get the IContextMenu Interface
    // We ask the parent folder for the Context Menu handler for the child item
    let context_menu: IContextMenu =
        unsafe { parent_folder.GetUIObjectOf(HWND(0 as _), &[child_pidl], None) }?;

    // 5. Create a fake Menu to capture the items
    let hmenu = unsafe { CreatePopupMenu() }?;

    // 6. Ask the interface to populate our menu
    // Flags: CMF_NORMAL (standard right click).
    // Use CMF_EXTENDEDVERBS if you want "Shift+RightClick" hidden items.
    unsafe { context_menu.QueryContextMenu(hmenu, 0, 1, 0x7FFF, CMF_NORMAL) }.ok()?;

    // 7. Iterate and Collect
    let entries = unsafe { walk_menu(hmenu, &context_menu) };

    // Cleanup
    unsafe { DestroyMenu(hmenu) }?;
    unsafe { CoTaskMemFree(Some(pidl as _)) };
    // Note: child_pidl is a pointer *into* pidl (usually), or managed by SHBindToParent logic,
    // but strict PIDL management is complex. In simple tools, letting OS cleanup on process exit is common.

    Ok(entries)
}

unsafe fn walk_menu(hmenu: HMENU, context_menu: &IContextMenu) -> Vec<ContextMenuEntry> {
    let count = unsafe { GetMenuItemCount(Some(hmenu)) };
    let mut entries = Vec::new();

    for i in 0..count {
        let mut info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_STRING | MIIM_SUBMENU | MIIM_ID | MIIM_FTYPE,
            ..Default::default()
        };

        // We need a buffer for the string
        let mut buffer = [0u16; 256];
        info.dwTypeData = PWSTR(buffer.as_mut_ptr());
        info.cch = 256;

        if unsafe { GetMenuItemInfoW(hmenu, i as u32, true, &mut info) }.is_ok() {
            // Check for separators
            if (info.fType & MFT_SEPARATOR) == MFT_SEPARATOR {
                entries.push(ContextMenuEntry {
                    id: 0,
                    label: "----------------".to_string(),
                    verb: "".to_string(),
                    sub_items: vec![],
                    is_separator: true,
                });
                continue;
            }

            let label = String::from_utf16_lossy(&buffer[..info.cch as usize]);

            // Try to get the "Verb" (Programmatic Name)
            let verb = unsafe { get_verb(context_menu, info.wID) };

            let mut sub_items = Vec::new();
            // Recursion for submenus (Expandos)
            if !info.hSubMenu.is_invalid() {
                sub_items = unsafe { walk_menu(info.hSubMenu, context_menu) };
            }

            entries.push(ContextMenuEntry {
                id: info.wID,
                label,
                verb,
                sub_items,
                is_separator: false,
            });
        }
    }
    entries
}

// Helper to try and get the verb string (e.g. "copy", "paste", "transcribe")
unsafe fn get_verb(context_menu: &IContextMenu, id: u32) -> String {
    // IDs usually start at 1 (the offset we passed to QueryContextMenu)
    // If the ID is very large or 0, it might be system reserved
    if !(1..=0x7FFF).contains(&id) {
        return "".to_string();
    }

    let offset = id - 1; // Convert Menu ID back to relative offset
    let mut buffer = [0u8; 256]; // GCS_VERBA uses ANSI usually

    // Try ANSI verb
    let hr = unsafe {
        context_menu.GetCommandString(
            offset.try_into().unwrap(),
            GCS_VERBA,
            None,
            PSTR(buffer.as_mut_ptr()),
            256,
        )
    };

    if hr.is_ok() {
        // quick and dirty conversion
        let len = buffer.iter().position(|&x| x == 0).unwrap_or(0);
        return String::from_utf8_lossy(&buffer[..len]).to_string();
    }

    // Try Unicode verb
    let mut buffer_w = [0u16; 256];
    let hr_w = unsafe {
        context_menu.GetCommandString(
            offset.try_into().unwrap(),
            GCS_VERBW,
            None,
            PSTR(buffer_w.as_mut_ptr() as _),
            256,
        )
    };

    if hr_w.is_ok() {
        let len = buffer_w.iter().position(|&x| x == 0).unwrap_or(0);
        return String::from_utf16_lossy(&buffer_w[..len]);
    }

    String::new()
}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() -> eyre::Result<()> {
        let path = file!();
        let entries = unsafe { super::get_context_menu_entries(path)? };
        for entry in entries {
            println!("{:?}", entry);
        }
        Ok(())
    }
}
