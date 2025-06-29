use crate::{
    BlockDevice, Error,
    filesystem::{
        BlockCache, DataAllocator, DataWriter, DirEntry, Directory, EraseFromDevice, File,
        FileHandle, Layout, Meta, Node, NodeHandle, NodeWriter, ReadFromDevice,
        StaticReadFromDevice, WriteToDevice, path,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    device: BlockCache<D>,
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
        let device = BlockCache::mount(device);
        Ok(Controller { device, directory, data_allocator })
    }

    pub fn unmount(self) -> D {
        self.device.unmount()
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        Meta::new().write_to_device(device)?;
        DirEntry::root().store(device, 0)
    }

    pub fn create(&mut self, file_path: &str, data: &[u8]) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        path::validate(file_path)?;

        let file_size = data.len();
        if file_size > Node::MAX_FILE_SIZE {
            return Err(Error::FileTooLarge);
        }

        let entry = self.directory.insert(&mut self.device, file_path)?;
        let file = File::new(*entry.name(), entry.file_addr());
        let node = self.data_allocator.allocate_node_data(&mut self.device, file_size)?;
        DataWriter::new(node.block_addrs(), data).write_to_device(&mut self.device)?;
        NodeWriter::new(file.addr(), &node).write_to_device(&mut self.device)?;
        file.write_to_device(&mut self.device)?;
        Ok(())
    }

    pub fn delete(&mut self, file_path: &str) -> Result<(), Error> {
        path::validate(file_path)?;

        let entry = self.directory.get(&mut self.device, file_path)?;
        let node_handle = NodeHandle::new(entry.file_addr());
        let file_handle = FileHandle::new(entry.file_addr());
        let node = node_handle.read_from_device(&mut self.device)?;

        node_handle.erase_from_device(&mut self.device)?;
        file_handle.erase_from_device(&mut self.device)?;
        self.directory.remove(&mut self.device, file_path)?;

        // Release data blocks only after metadata is fully erased.
        self.data_allocator.release_node_data(&mut self.device, &node)?;
        Ok(())
    }

    pub fn count_files(&mut self) -> Result<usize, Error> {
        self.directory.count_files(&mut self.device)
    }

    pub fn print_tree(&mut self) -> Result<(), Error> {
        self.directory.print_tree(&mut self.device)
    }
}
