use crate::audio::TeamyImmDeviceIcon;
use crate::shell::property_store::PropVariantExt;
use eyre::Context;
use std::ops::Deref;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::core::GUID;

// DEVPKEY_Device_IconPath
pub const PKEY_DEVICE_ICON: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x259abffc_507a_4ce8_8c10_9640b8a1c907),
    pid: 10,
};

// DEVPKEY_DeviceClass_IconPath
pub const PKEY_DEVICE_CLASS_ICON: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x259abffc_507a_4ce8_8c10_9640b8a1c907),
    pid: 12,
};

#[derive(Debug)]
pub struct TeamyImmDeviceIconPath(pub String);
impl TeamyImmDeviceIconPath {
    pub fn new(path: String) -> Self {
        Self(path)
    }
    pub fn from_property_store(property_store: &IPropertyStore) -> eyre::Result<Self> {
        let property = unsafe { property_store.GetValue(&PKEY_DEVICE_ICON) }
            .or_else(|_| unsafe { property_store.GetValue(&PKEY_DEVICE_CLASS_ICON) })
            .wrap_err_with(|| "Failed getting either PKEY_DEVICE_ICON PKEY_DEVICE_CLASS_ICON")?;
        let property = property.interpret_string_value()?;
        Ok(TeamyImmDeviceIconPath::new(property))
    }
    pub fn load_device_icon(&self) -> eyre::Result<TeamyImmDeviceIcon> {
        let icon = crate::hicon::load_icon_from_path(&self.0)?;
        Ok(icon)
    }
}
impl Default for TeamyImmDeviceIconPath {
    fn default() -> Self {
        Self(format!(
            "{system_root}\\system32\\mmres.dll,-3012",
            system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
        ))
    }
}
impl Deref for TeamyImmDeviceIconPath {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
