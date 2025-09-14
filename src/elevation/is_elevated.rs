use std::mem::size_of;
use std::ops::DerefMut;
use std::sync::OnceLock;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::GetTokenInformation;
use windows::Win32::Security::TOKEN_ELEVATION;
use windows::Win32::Security::TOKEN_QUERY;
use windows::Win32::Security::TokenElevation;
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::Threading::OpenProcessToken;
use windows::core::Owned;

static IS_ELEVATED: OnceLock<bool> = OnceLock::new();

/// Checks if the current process is running with elevated privileges.
pub fn is_elevated() -> bool {
    *IS_ELEVATED.get_or_init(|| unsafe {
        let mut token_handle = Owned::new(HANDLE::default());
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, token_handle.deref_mut()).is_err() {
            eprintln!("Failed to open process token. Error: {:?}", GetLastError());
            return false;
        }

        let mut elevation = TOKEN_ELEVATION::default();
        let mut return_length = 0;

        let ok = GetTokenInformation(
            *token_handle,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        )
        .is_ok();

        let result = if ok {
            elevation.TokenIsElevated != 0
        } else {
            eprintln!(
                "Failed to get token information. Error: {:?}",
                GetLastError()
            );
            false
        };

        result
    })
}
