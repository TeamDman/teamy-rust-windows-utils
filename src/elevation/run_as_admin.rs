
use eyre::Context;
use std::ffi::OsString;
use windows::Win32::UI::Shell::SEE_MASK_NOCLOSEPROCESS;
use windows::Win32::UI::Shell::SHELLEXECUTEINFOW;
use windows::Win32::UI::Shell::ShellExecuteExW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::elevation::ElevatedChildProcess;
use crate::invocation::Invocable;
use crate::string::EasyPCWSTR;

/// Runs an invocable with administrative privileges using ShellExecuteExW.
pub fn run_as_admin(invocable: &impl Invocable) -> eyre::Result<ElevatedChildProcess> {
    // Build a single space-separated string of arguments
    let params: OsString = invocable
        .args()
        .into_iter()
        .fold(OsString::new(), |mut acc, arg| {
            acc.push(arg);
            acc.push(" ");
            acc
        });

    // ---------------- ShellExecuteExW ----------------
    let verb = "runas".easy_pcwstr()?;
    let file = invocable.executable().easy_pcwstr()?;
    let params = params.easy_pcwstr()?;
    unsafe {
        let mut sei = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: params.as_ptr(),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        ShellExecuteExW(&mut sei).wrap_err("Failed to run as administrator")?;
        Ok(ElevatedChildProcess {
            h_process: sei.hProcess,
        })
    }
}
