use eyre::Result;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

/// RAII Guard for COM Initialization.
///
/// Calls `CoInitializeEx` on creation and `CoUninitialize` on drop if initialization was successful
/// (or if it was already initialized with a compatible mode, incrementing the refcount).
pub struct ComGuard {
    should_uninitialize: bool,
}

impl ComGuard {
    pub fn new() -> Result<Self> {
        unsafe {
            let result = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            if result.is_ok() {
                // S_OK: Initialized successfully.
                // S_FALSE: Already initialized with same mode. Ref count incremented.
                // In both cases, we must balance with CoUninitialize.
                Ok(Self {
                    should_uninitialize: true,
                })
            } else if result == RPC_E_CHANGED_MODE {
                // Already initialized with a different mode (e.g. MTA).
                // We cannot change it, and we should not uninitialize it.
                // We proceed hoping the existing mode is compatible enough.
                Ok(Self {
                    should_uninitialize: false,
                })
            } else {
                // Actual error (e.g. out of memory)
                Err(windows::core::Error::from(result).into())
            }
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
