
use image::RgbaImage;
use std::ops::Deref;
pub struct TeamyImmDeviceIcon(pub RgbaImage);

impl TeamyImmDeviceIcon {
    pub fn new(image: RgbaImage) -> Self {
        Self(image)
    }
}

impl Default for TeamyImmDeviceIcon {
    fn default() -> Self {
        todo!()
    }
}

impl Deref for TeamyImmDeviceIcon {
    type Target = RgbaImage;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
