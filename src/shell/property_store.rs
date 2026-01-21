use eyre::bail;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Variant::VT_LPWSTR;

pub trait PropVariantExt {
    fn interpret_string_value(&self) -> eyre::Result<String>;
}
impl PropVariantExt for PROPVARIANT {
    fn interpret_string_value(&self) -> eyre::Result<String> {
        let varenum = unsafe { self.Anonymous.Anonymous.vt };
        let VT_LPWSTR = varenum else {
            bail!("PROPVARIANT is not of type VT_LPWSTR, was {varenum:?} instead",);
        };
        let pwstr = unsafe { self.Anonymous.Anonymous.Anonymous.pwszVal };
        if pwstr.is_null() {
            bail!("PROPVARIANT contains null pointer for string value");
        }
        Ok(unsafe { pwstr.to_string() }?)
    }
}
