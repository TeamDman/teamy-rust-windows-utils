use crate::audio::TeamyImmDeviceIcon;
use crate::audio::imm_device_id::TeamyImmDeviceId;

/// Interface MultiMedia Device
pub struct TeamyImmDevice {
    pub id: TeamyImmDeviceId,
    pub name: String,
    pub is_default: bool,
    pub icon: Option<TeamyImmDeviceIcon>,
}
