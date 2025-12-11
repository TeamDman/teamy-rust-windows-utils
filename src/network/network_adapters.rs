use eyre::bail;
use std::marker::PhantomData;
use windows::Win32::Foundation::ERROR_ADDRESS_NOT_ASSOCIATED;
use windows::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
use windows::Win32::Foundation::ERROR_INVALID_PARAMETER;
use windows::Win32::Foundation::ERROR_NO_DATA;
use windows::Win32::Foundation::ERROR_NOT_ENOUGH_MEMORY;
use windows::Win32::Foundation::NO_ERROR;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::NetworkManagement::IpHelper::GAA_FLAG_INCLUDE_ALL_INTERFACES;
use windows::Win32::NetworkManagement::IpHelper::GetAdaptersAddresses;
use windows::Win32::NetworkManagement::IpHelper::IP_ADAPTER_ADDRESSES_LH;
use windows::Win32::Networking::WinSock::AF_UNSPEC;

const WORKING_BUFFER_SIZE: usize = 15 * 1024;
const MAX_RESIZE_ATTEMPTS: usize = 4;

/// Owns the backing buffer returned by `GetAdaptersAddresses` so that the
/// pointed-to fields inside each `IP_ADAPTER_ADDRESSES_LH` stay valid.
#[derive(Default)]
pub struct NetworkAdapters {
    buffer: Vec<u8>,
}
impl core::fmt::Debug for NetworkAdapters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkAdapters")
            .field("count", &self.iter().count())
            .finish()
    }
}

impl NetworkAdapters {
    /// Allocates the working buffer and immediately populates it with a
    /// snapshot from `GetAdaptersAddresses`.
    pub fn new() -> eyre::Result<Self> {
        let mut adapters = Self::default();
        adapters.refresh()?;
        Ok(adapters)
    }

    /// Refreshes the adapter list in-place, reusing the existing allocation to
    /// avoid churn when polling frequently.
    pub fn refresh(&mut self) -> eyre::Result<()> {
        if self.buffer.is_empty() {
            self.buffer.resize(WORKING_BUFFER_SIZE, 0);
        }

        let mut attempts = 0usize;

        loop {
            let mut buffer_size = self.buffer.len() as u32;
            let adapter_ptr = self.buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;

            let adapter_ptr_mut = unsafe { &mut *adapter_ptr };
            let status = unsafe {
                GetAdaptersAddresses(
                    AF_UNSPEC.0 as u32,
                    GAA_FLAG_INCLUDE_ALL_INTERFACES,
                    None,
                    Some(adapter_ptr_mut),
                    &mut buffer_size,
                )
            };

            match WIN32_ERROR(status) {
                ERROR_BUFFER_OVERFLOW => {
                    attempts += 1;
                    if attempts >= MAX_RESIZE_ATTEMPTS {
                        bail!(
                            "GetAdaptersAddresses kept requesting a larger buffer (required {buffer_size} bytes)"
                        );
                    }
                    self.buffer.resize(buffer_size as usize, 0);
                    continue;
                }
                ERROR_NO_DATA => {
                    self.buffer.clear();
                    return Ok(());
                }
                ERROR_ADDRESS_NOT_ASSOCIATED => {
                    bail!("No addresses have been associated with the requested adapters yet")
                }
                ERROR_INVALID_PARAMETER => {
                    bail!("Invalid parameter passed to GetAdaptersAddresses")
                }
                ERROR_NOT_ENOUGH_MEMORY => {
                    bail!("Not enough memory to enumerate network adapters")
                }
                NO_ERROR => {
                    let used = buffer_size as usize;
                    self.buffer.truncate(used);
                    return Ok(());
                }
                other => {
                    let message = other.to_hresult().message();
                    bail!("GetAdaptersAddresses failed: {message}");
                }
            }
        }
    }

    fn head_ptr(&self) -> *const IP_ADAPTER_ADDRESSES_LH {
        if self.buffer.is_empty() {
            std::ptr::null()
        } else {
            self.buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH
        }
    }

    pub fn iter(&self) -> NetworkAdapterIter<'_> {
        NetworkAdapterIter {
            next: self.head_ptr(),
            _marker: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }
}

pub struct NetworkAdapterIter<'a> {
    next: *const IP_ADAPTER_ADDRESSES_LH,
    _marker: PhantomData<&'a IP_ADAPTER_ADDRESSES_LH>,
}

impl<'a> Iterator for NetworkAdapterIter<'a> {
    type Item = &'a IP_ADAPTER_ADDRESSES_LH;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_null() {
            return None;
        }

        let current = self.next;
        let next_ptr = unsafe { (*current).Next };
        self.next = next_ptr;
        let item = unsafe { &*current };
        Some(item)
    }
}

impl<'a> IntoIterator for &'a NetworkAdapters {
    type Item = &'a IP_ADAPTER_ADDRESSES_LH;
    type IntoIter = NetworkAdapterIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod test {
    use crate::network::NetworkAdapterExt;
    use crate::network::OperStatusExt;

    #[test]
    fn enumerates_adapters() -> eyre::Result<()> {
        let mut adapters = super::NetworkAdapters::new()?;
        let count = adapters.iter().count();
        let adapter_display = adapters
            .iter()
            .map(|adapter| {
                (
                    adapter.display_name(),
                    unsafe { adapter.AdapterName.display() }.to_string(),
                    adapter.peOperStatus.display(),
                )
            })
            .collect::<Vec<_>>();
        println!("Adapters: {:#?}", adapter_display);
        println!("Enumerated {count} adapters");
        assert!(count > 0, "expected at least one adapter");
        adapters.refresh()?;
        Ok(())
    }
}
