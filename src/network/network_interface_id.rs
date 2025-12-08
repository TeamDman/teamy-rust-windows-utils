use std::fmt;
use std::mem;
use windows::Win32::NetworkManagement::IpHelper::IP_ADAPTER_ADDRESSES_LH;
use windows::Win32::NetworkManagement::IpHelper::MIB_IF_ROW2;
use windows::Win32::NetworkManagement::Ndis::NET_LUID_LH;

/// Identifier for a network interface. Prefer `Luid` when available and fall back
/// to the legacy interface index when required by older APIs.
// there's an api to convert between these, haven't added support for that yet
#[derive(Clone, Copy)]
pub enum NetworkInterfaceId {
    Index(u32),
    Luid(NET_LUID_LH),
}

impl NetworkInterfaceId {
    pub fn apply_to_row(self, row: &mut MIB_IF_ROW2) {
        match self {
            NetworkInterfaceId::Index(index) => row.InterfaceIndex = index,
            NetworkInterfaceId::Luid(luid) => row.InterfaceLuid = luid,
        }
    }
}

impl fmt::Debug for NetworkInterfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkInterfaceId::Index(index) => f.debug_tuple("Index").field(index).finish(),
            NetworkInterfaceId::Luid(luid) => {
                let bits: u64 = unsafe { mem::transmute_copy(luid) };
                f.debug_tuple("Luid").field(&bits).finish()
            }
        }
    }
}

impl PartialEq for NetworkInterfaceId {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NetworkInterfaceId::Index(a), NetworkInterfaceId::Index(b)) => a == b,
            (NetworkInterfaceId::Luid(a), NetworkInterfaceId::Luid(b)) => {
                let lhs: u64 = unsafe { mem::transmute_copy(a) };
                let rhs: u64 = unsafe { mem::transmute_copy(b) };
                lhs == rhs
            }
            _ => false,
        }
    }
}

impl Eq for NetworkInterfaceId {}

impl From<u32> for NetworkInterfaceId {
    fn from(value: u32) -> Self {
        NetworkInterfaceId::Index(value)
    }
}

impl From<NET_LUID_LH> for NetworkInterfaceId {
    fn from(value: NET_LUID_LH) -> Self {
        NetworkInterfaceId::Luid(value)
    }
}

impl From<&IP_ADAPTER_ADDRESSES_LH> for NetworkInterfaceId {
    fn from(value: &IP_ADAPTER_ADDRESSES_LH) -> Self {
        NetworkInterfaceId::Luid(value.Luid)
    }
}
