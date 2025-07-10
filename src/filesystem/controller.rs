use crate::{
    BlockDevice, Error,
    filesystem::{
        EraseFrom, Layout, LoadFrom,
        allocator::{Allocator, DataAllocator},
        cache::BlockCache,
        data_reader::DataReader,
        directory::Tree,
        file::File,
        meta::Meta,
        node::Node,
        paths, storage,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    device: BlockCache<D>,
    directory: Tree,
    allocator: Allocator,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn mount(mut device: D) -> Result<Self, Error> {
        if Meta::load_from(&mut device, 0)? != Meta::new() {
            return Err(Error::UnsupportedDevice);
        }
        let device = BlockCache::mount(device);
        let directory = Tree::new(Layout::TREE_BITMAP);
        let allocator = Allocator::new(Layout::DATA_BITMAP);
        Ok(Self { device, directory, allocator })
    }

    pub fn unmount(self) -> D {
        self.device.unmount()
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        storage::store(device, 0, &Meta::new())?;
        Tree::new(Layout::TREE_BITMAP).format(device)?;
        Ok(())
    }

    pub fn create(&mut self, file_path: &str, data: &[u8]) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        paths::validate(file_path)?;

        let file_size = data.len();
        if file_size > Node::MAX_FILE_SIZE {
            return Err(Error::FileTooLarge);
        }

        let entry = self.directory.insert_file(&mut self.device, file_path)?;
        let file = File::new(*entry.name(), entry.addr());
        let node = self.allocator.allocate_node_data(&mut self.device, file_size)?;
        storage::store_data(&mut self.device, node.data_addrs(), data)?;
        storage::store(&mut self.device, file.node_addr(), &node)?;
        storage::store(&mut self.device, file.node_addr(), &file)?;
        Ok(())
    }

    pub fn delete(&mut self, file_path: &str) -> Result<(), Error> {
        paths::validate(file_path)?;

        let entry = self.directory.get_file(&mut self.device, file_path)?;
        let node = Node::load_from(&mut self.device, entry.addr())?;
        Node::erase_from(&mut self.device, entry.addr())?;
        File::erase_from(&mut self.device, entry.addr())?;
        self.directory.remove_file(&mut self.device, file_path)?;
        self.directory.prune(&mut self.device, 0)?;

        // Release data blocks only after metadata is fully erased.
        self.allocator.release_node_data(&mut self.device, &node)?;
        Ok(())
    }

    pub fn open(&mut self, file_path: &str) -> Result<DataReader<D>, Error> {
        paths::validate(file_path)?;

        let entry = self.directory.get_file(&mut self.device, file_path)?;
        let node = Node::load_from(&mut self.device, entry.addr())?;
        Ok(DataReader::new(&mut self.device, node))
    }

    pub fn count_files(&mut self) -> Result<usize, Error> {
        self.directory.count_files(&mut self.device)
    }

    pub fn count_dirs(&mut self) -> Result<usize, Error> {
        self.directory.count_dirs(&mut self.device)
    }

    pub fn free_data_blocks(&mut self) -> Result<usize, Error> {
        self.allocator.count_free_addresses(&mut self.device)
    }

    #[cfg(feature = "std")]
    pub fn print_tree(&mut self, base_path: &str, depth: usize) -> Result<(), Error> {
        use crate::filesystem::directory::tree_printer;
        tree_printer::print_tree_stdout(&mut self.device, base_path, depth)
    }

    #[cfg(feature = "std")]
    pub fn print_disk_layout(&self) {
        Layout::print_disk_layout();
    }
}
