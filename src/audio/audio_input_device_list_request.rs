use crate::audio::TeamyImmDeviceIconPath;
use crate::audio::imm_device::TeamyImmDevice;
use crate::audio::imm_device_id::TeamyImmDeviceId;
use crate::com::com_guard::ComGuard;
use windows::Win32::Devices::Properties::DEVPKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE;
use windows::Win32::Media::Audio::IMMDevice;
use windows::Win32::Media::Audio::IMMDeviceCollection;
use windows::Win32::Media::Audio::IMMDeviceEnumerator;
use windows::Win32::Media::Audio::MMDeviceEnumerator;
use windows::Win32::Media::Audio::eCapture;
use windows::Win32::Media::Audio::eMultimedia;
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::Win32::System::Com::CoCreateInstance;
use windows::Win32::System::Com::STGM_READ;
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;

pub fn list_audio_input_devices() -> eyre::Result<Vec<TeamyImmDevice>> {
    let _com_guard = ComGuard::new()?;

    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }?;

    let default_device = unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, eMultimedia) }?;
    let default_device_id = TeamyImmDeviceId::new(unsafe { default_device.GetId()? })?;

    let collection: IMMDeviceCollection =
        unsafe { enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) }?;
    let count = unsafe { collection.GetCount() }?;

    let mut rtn = Vec::new();

    for i in 0..count {
        // Get the device
        let device: IMMDevice = unsafe { collection.Item(i)? };

        // Get the device ID
        let device_id = TeamyImmDeviceId::new(unsafe { device.GetId()? })?;

        // Determine if the device matches our default device
        let is_default = default_device_id == device_id;

        // Get the device friendly name
        let device_property_store: IPropertyStore = unsafe { device.OpenPropertyStore(STGM_READ)? };
        let name = unsafe {
            device_property_store
                .GetValue(&DEVPKEY_Device_FriendlyName as *const _ as *const PROPERTYKEY)
        }
        .map(|prop_variant| prop_variant.to_string())
        .unwrap_or_else(|_| "(Unknown Device)".to_string());

        // Get the device icon path
        let device_icon = TeamyImmDeviceIconPath::from_property_store(&device_property_store)
            .unwrap_or_default()
            .load_device_icon()
            .ok();

        // Add device to the list of results
        rtn.push(TeamyImmDevice {
            id: device_id,
            name,
            is_default,
            icon: device_icon,
        });
    }
    Ok(rtn)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn it_works() -> eyre::Result<()> {
        let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_output");
        std::fs::create_dir_all(&output_dir)?;

        let devices = crate::audio::audio_input_device_list_request::list_audio_input_devices()?;
        for (i, device) in devices.iter().enumerate() {
            // Print device info
            println!(
                "Device: {:?} {:?}{}",
                device.id,
                device.name,
                if device.is_default { " (default)" } else { "" }
            );

            // Save device icon if available
            if let Some(ref icon) = device.icon {
                let icon_path = output_dir.join(format!("device_icon_{i}.png"));
                icon.0.save(&icon_path)?;
                println!("  Icon saved to {:?}", icon_path);
            } else {
                println!("  No icon available");
            }
        }
        Ok(())
    }
}
