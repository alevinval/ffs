use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Block, Deserializable, File, Layout, ReadFromDevice},
};

/// A handle to a file in the filesystem, identified by its address.
///
/// This struct does not own the file data, but provides a way to load the file
/// from a block device using the stored address.
pub struct FileHandle {
    /// The addr of the file in the filesystem.
    addr: Addr,
}

impl FileHandle {
    /// Creates a new [`FileHandle`] for the given address.
    pub fn new(addr: Addr) -> Self {
        Self { addr }
    }
}

impl<D: BlockDevice> ReadFromDevice<D> for FileHandle {
    type Item = File;

    fn read_from_device(&self, device: &mut D) -> Result<Self::Item, Error> {
        let sector = Layout::FILE.nth(self.addr);
        let mut block = Block::new();
        device.read_block(sector, &mut block)?;
        File::deserialize(&mut block.reader())
    }
}

#[cfg(test)]
mod test {

    use crate::{
        filesystem::{Serializable, WriteToDevice},
        test_utils::MockDevice,
    };

    use super::*;

    #[test]
    fn read_from_device() -> Result<(), Error> {
        let mut device = MockDevice::new();
        let expected = File::new("some-file.txt".into(), 123);
        let mut block = Block::new();
        expected.serialize(&mut block.writer())?;
        expected.write_to_device(&mut device)?;

        let handle = FileHandle::new(123);
        let actual = handle.read_from_device(&mut device)?;

        assert_eq!(expected, actual);

        Ok(())
    }
}
