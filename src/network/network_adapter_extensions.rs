use crate::network::NetworkInterfaceId;
use crate::network::NetworkInterfaceMonitor;
use std::borrow::Cow;
use windows::Win32::NetworkManagement::IpHelper::IP_ADAPTER_ADDRESSES_LH;

pub trait NetworkAdapterExt {
    fn id(&self) -> NetworkInterfaceId;
    fn monitor(&self) -> eyre::Result<NetworkInterfaceMonitor>;
    fn display_name(&self) -> Cow<'_, str>;
}
impl NetworkAdapterExt for IP_ADAPTER_ADDRESSES_LH {
    fn id(&self) -> NetworkInterfaceId {
        NetworkInterfaceId::from(self)
    }
    fn monitor(&self) -> eyre::Result<NetworkInterfaceMonitor> {
        NetworkInterfaceMonitor::new(self)
    }
    fn display_name(&self) -> Cow<'_, str> {
        if self.FriendlyName.is_null() {
            Cow::Borrowed("")
        } else {
            Cow::Owned(unsafe { self.FriendlyName.display() }.to_string())
        }
    }
}
