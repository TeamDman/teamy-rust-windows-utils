use std::os::windows::fs::MetadataExt;
use std::path::Path;
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;

#[allow(unused)]
pub trait IsAvailableOnDevice {
    fn is_available_on_device(&self) -> eyre::Result<bool>;
}
impl<T: AsRef<Path>> IsAvailableOnDevice for T {
    fn is_available_on_device(&self) -> eyre::Result<bool> {
        let path = self.as_ref();
        let stat = path.metadata()?;
        Ok((FILE_FLAGS_AND_ATTRIBUTES(stat.file_attributes())
            & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            .0
            == 0)
    }
}

#[cfg(test)]
mod test {
    use crate::file::IsAvailableOnDevice;

    #[test]
    fn it_works() -> eyre::Result<()> {
        let dir = r#"C:\Users\TeamD\OneDrive\Memes"#;
        let children = std::fs::read_dir(dir)?;
        for child in children {
            let child = child?;
            let path = child.path();
            let is_available = path.is_available_on_device()?;
            println!("{path:?} => {is_available}");
        }

        Ok(())
    }
}
