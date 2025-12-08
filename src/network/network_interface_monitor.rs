use crate::network::NetworkInterfaceId;
use eyre::bail;
use std::fmt;
use windows::Win32::Foundation::NO_ERROR;
use windows::Win32::NetworkManagement::IpHelper::GetIfEntry2;
use windows::Win32::NetworkManagement::IpHelper::MIB_IF_ROW2;
use windows::Win32::NetworkManagement::Ndis::IF_OPER_STATUS;

/// Refreshable snapshot of a single interface's `MIB_IF_ROW2` counters/state.
pub struct NetworkInterfaceMonitor {
    id: NetworkInterfaceId,
    row: MIB_IF_ROW2,
}

impl fmt::Debug for NetworkInterfaceMonitor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkInterfaceMonitor")
            .field("id", &self.id)
            .finish()
    }
}

impl NetworkInterfaceMonitor {
    pub fn new(id: impl Into<NetworkInterfaceId>) -> eyre::Result<Self> {
        let mut monitor = Self {
            id: id.into(),
            row: MIB_IF_ROW2::default(),
        };
        monitor.refresh()?;
        Ok(monitor)
    }

    pub fn refresh(&mut self) -> eyre::Result<()> {
        let mut row = MIB_IF_ROW2::default();
        self.id.apply_to_row(&mut row);
        let status = unsafe { GetIfEntry2(&mut row) };
        if status != NO_ERROR {
            let message = status.to_hresult().message();
            bail!("GetIfEntry2 failed: {message}");
        }
        self.row = row;
        Ok(())
    }

    pub fn oper_status(&self) -> IF_OPER_STATUS {
        self.row.OperStatus
    }

    pub fn row(&self) -> &MIB_IF_ROW2 {
        &self.row
    }

    pub fn id(&self) -> NetworkInterfaceId {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::super::NetworkAdapters;
    use crate::network::NetworkAdapterExtensions;

    #[test]
    fn monitor_tracks_single_interface() -> eyre::Result<()> {
        let adapters = NetworkAdapters::new()?;
        let interface = adapters
            .iter()
            .next()
            .expect("expected at least one network adapter");
        let interface_id = interface.id();
        let interface_name = interface.display_name();
        println!("Testing interface monitor for adapter: {}", interface_name);
        let mut monitor = interface.monitor()?;
        monitor.refresh()?;
        assert_eq!(monitor.id(), interface_id);
        assert!(monitor.row().InterfaceIndex != 0);
        Ok(())
    }
}
