use eyre::Context;
use windows::core::w;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Console::{
    GetStdHandle, SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};
use windows::Win32::Foundation::{CloseHandle, HANDLE};

/// Returns the current STDOUT handle, erroring if it's invalid.
pub fn get_console_output_handle() -> eyre::Result<HANDLE> {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE)
            .wrap_err("Failed to get standard output handle")?;
        if handle.is_invalid() {
            Err(windows::core::Error::from_thread())
                .wrap_err("STD_OUTPUT_HANDLE is invalid")
        } else {
            Ok(handle)
        }
    }
}

/// Rebinds STDOUT/STDERR/STDIN to the current console using CONOUT$/CONIN$.
/// Closes previously set std handles to avoid keeping the console host alive.
pub fn rebind_std_handles_to_console() -> eyre::Result<()> {
    unsafe {
        // Capture previous std handles so we can close them after switching
        let prev_out = GetStdHandle(STD_OUTPUT_HANDLE).unwrap_or_default();
        let prev_err = GetStdHandle(STD_ERROR_HANDLE).unwrap_or_default();
        let prev_in = GetStdHandle(STD_INPUT_HANDLE).unwrap_or_default();

        // OUTPUT/ERROR → CONOUT$
        let conout = CreateFileW(
            w!("CONOUT$"),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .wrap_err("Failed to open CONOUT$")?;

        SetStdHandle(STD_OUTPUT_HANDLE, conout)
            .wrap_err("Failed to set STDOUT to CONOUT$")?;
        SetStdHandle(STD_ERROR_HANDLE, conout)
            .wrap_err("Failed to set STDERR to CONOUT$")?;

        // Close previous handles if valid and different from new
        if !prev_out.is_invalid() && prev_out != conout {
            let _ = CloseHandle(prev_out);
        }
        if !prev_err.is_invalid() && prev_err != conout {
            let _ = CloseHandle(prev_err);
        }

        // INPUT → CONIN$ (best-effort)
        if let Ok(conin) = CreateFileW(
            w!("CONIN$"),
            FILE_GENERIC_READ.0 | FILE_GENERIC_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        ) {
            let _ = SetStdHandle(STD_INPUT_HANDLE, conin);
            if !prev_in.is_invalid() && prev_in != conin {
                let _ = CloseHandle(prev_in);
            }
        }
    }
    Ok(())
}

/// Unbind and close current STD handles so the console host can close immediately when detaching.
pub fn unbind_and_close_std_handles_for_detach() {
    unsafe {
        let out = GetStdHandle(STD_OUTPUT_HANDLE).unwrap_or_default();
        let err = GetStdHandle(STD_ERROR_HANDLE).unwrap_or_default();
        let inp = GetStdHandle(STD_INPUT_HANDLE).unwrap_or_default();

        // Reset std handles to null
        let _ = SetStdHandle(STD_OUTPUT_HANDLE, HANDLE::default());
        let _ = SetStdHandle(STD_ERROR_HANDLE, HANDLE::default());
        let _ = SetStdHandle(STD_INPUT_HANDLE, HANDLE::default());

        // Close any valid handles so we don't keep the console alive
        if !out.is_invalid() {
            let _ = CloseHandle(out);
        }
        if !err.is_invalid() {
            let _ = CloseHandle(err);
        }
        if !inp.is_invalid() {
            let _ = CloseHandle(inp);
        }
    }
}
