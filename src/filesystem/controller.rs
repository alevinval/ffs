use crate::{
    BlockDevice, Error,
    filesystem::{
        EraseFrom, Layout, LoadFrom, LoadFromStatic, Store,
        cache::BlockCache,
        data_allocator::DataAllocator,
        data_writer::DataWriter,
        directory::{DirNode, DirTree},
        file::File,
        meta::Meta,
        node::Node,
        node_writer::NodeWriter,
        path,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    device: BlockCache<D>,
    directory: DirTree,
    data_allocator: DataAllocator,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn mount(mut device: D) -> Result<Self, Error> {
        if Meta::load_from(&mut device)? != Meta::new() {
            return Err(Error::Unsupported);
        }
        let directory = DirTree {};
        let data_allocator = DataAllocator::new(Layout::FREE);
        let device = BlockCache::mount(device);
        Ok(Self { device, directory, data_allocator })
    }

    pub fn unmount(self) -> D {
        self.device.unmount()
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        Meta::new().store(device)?;
        DirNode::new().store(device, 0)
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

        let file_ref = self.directory.insert_file(&mut self.device, file_path)?;
        let file = File::new(*file_ref.name(), file_ref.addr());
        let node = self.data_allocator.allocate_node_data(&mut self.device, file_size)?;
        DataWriter::new(node.block_addrs(), data).store(&mut self.device)?;
        NodeWriter::new(file.node_addr(), &node).store(&mut self.device)?;
        file.store(&mut self.device)?;
        Ok(())
    }

    pub fn delete(&mut self, file_path: &str) -> Result<(), Error> {
        path::validate(file_path)?;

        let file_ref = self.directory.get_file(&mut self.device, file_path)?;
        let (file_handle, node_handle) = file_ref.get_handles();
        let node = node_handle.load_from(&mut self.device)?;

        node_handle.erase_from(&mut self.device)?;
        file_handle.erase_from(&mut self.device)?;
        self.directory.remove_file(&mut self.device, file_path)?;

        // Release data blocks only after metadata is fully erased.
        self.data_allocator.release_node_data(&mut self.device, &node)?;
        Ok(())
    }

    pub fn count_files(&mut self) -> Result<usize, Error> {
        self.directory.count_files(&mut self.device)
    }

    pub fn free_data_blocks(&mut self) -> Result<usize, Error> {
        self.data_allocator.count_free_addresses(&mut self.device)
    }

    #[cfg(feature = "std")]
    pub fn print_tree(&mut self) -> Result<(), Error> {
        self.directory.print_tree(&mut self.device)
    }
}
