use crate::{
    BlockDevice, Error,
    filesystem::{
        DataAllocator, DataWriter, Directory, EntryIter, EraseFromDevice, File, FileHandle,
        FileName, Layout, MAX_FILENAME_LEN, Meta, Node, NodeHandle, NodeWriter, ReadFromDevice,
        StaticReadFromDevice, WriteToDevice,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    device: D,
    directory: Directory,
    data_allocator: DataAllocator,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn mount(mut device: D) -> Result<Controller<D>, Error> {
        if Meta::read_from_device(&mut device)? != Meta::new() {
            return Err(Error::Unsupported);
        }
        let directory = Directory {};
        let data_allocator = DataAllocator::new(Layout::FREE);
        Ok(Self { directory, data_allocator, device })
    }

    pub fn unmount(self) -> D {
        self.device
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        Meta::new().write_to_device(device)
    }

    pub fn create(&mut self, file_name: &str, data: &[u8]) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        let file_name = FileName::new(file_name)?;
        let file_size = data.len();

        if file_size > Node::MAX_FILE_SIZE {
            return Err(Error::FileTooLarge);
        }

        if file_name.len() > MAX_FILENAME_LEN {
            return Err(Error::FileNameTooLong);
        }

        if self.directory.file_exists(&mut self.device, &file_name) {
            return Err(Error::FileAlreadyExists);
        }

        let entry = self.directory.add_file(&mut self.device, file_name)?;
        let file = File::new(file_name, entry.file_addr());
        let node = self.data_allocator.allocate_node_data(&mut self.device, file_size)?;
        DataWriter::new(node.block_addrs(), data).write_to_device(&mut self.device)?;
        NodeWriter::new(file.addr(), &node).write_to_device(&mut self.device)?;
        file.write_to_device(&mut self.device)?;
        Ok(())
    }

    pub fn delete(&mut self, file_name: &str) -> Result<(), Error> {
        let file_name = FileName::new(file_name)?;

        let entry = self.directory.find_file(&mut self.device, &file_name)?;
        let node_handle = NodeHandle::new(entry.file_addr());
        let file_handle = FileHandle::new(entry.file_addr());
        let node = node_handle.read_from_device(&mut self.device)?;

        node_handle.erase_from_device(&mut self.device)?;
        file_handle.erase_from_device(&mut self.device)?;
        self.directory.remove_file(&mut self.device, &file_name)?;

        // Release data blocks only after metadata is fully erased.
        self.data_allocator.release_node_data(&mut self.device, &node)?;
        Ok(())
    }

    pub fn entries(&mut self) -> EntryIter<D> {
        self.directory.iter(&mut self.device)
    }

    pub fn device(&mut self) -> &mut D {
        &mut self.device
    }
}
