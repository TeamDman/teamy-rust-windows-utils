use crate::string::EasyPCWSTR;
use std::ops::Deref;

#[derive(Debug, Eq, PartialEq)]
pub struct TeamyImmDeviceId(pub String);
impl Deref for TeamyImmDeviceId {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TeamyImmDeviceId {
    pub fn new(id: impl EasyPCWSTR) -> eyre::Result<Self> {
        let guard = id.easy_pcwstr()?;
        Ok(Self(guard.to_string().unwrap_or_default()))
    }
}

#[cfg(test)]
mod test {
    use crate::audio::imm_device_id::TeamyImmDeviceId;
    use widestring::U16CString;

    #[test]
    fn it_works() -> eyre::Result<()> {
        let x = U16CString::new();
        TeamyImmDeviceId::new(x)?;
        let y = "asd";
        TeamyImmDeviceId::new(y)?;
        Ok(())
    }
}
