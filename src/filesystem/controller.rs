use crate::{
    BlockDevice, Error,
    filesystem::{
        Layout,
        allocator::{Allocator, DataAllocator},
        cache::BlockCache,
        data_reader::DataReader,
        file::File,
        meta::Meta,
        node::Node,
        paths, storage,
        tree::Tree,
    },
};

#[derive(Debug)]
pub struct Controller<D>
where
    D: BlockDevice,
{
    device: BlockCache<D>,
    data_allocator: Allocator,
    tree_allocator: Allocator,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn mount(mut device: D) -> Result<Self, Error> {
        let meta: Meta = storage::load(&mut device, 0)?;
        if meta != Meta::new() {
            return Err(Error::UnsupportedDevice);
        }
        let device = BlockCache::mount(device);
        let data_allocator = Allocator::new(Layout::DATA_BITMAP);
        let tree_allocator = Allocator::new(Layout::TREE_BITMAP);
        Ok(Self { device, data_allocator, tree_allocator })
    }

    pub fn unmount(self) -> D {
        self.device.unmount()
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        storage::store(device, 0, &Meta::new())?;
        Tree::format(device, &mut Allocator::new(Layout::TREE_BITMAP))?;
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

        let entry = Tree::insert_file(&mut self.device, &mut self.tree_allocator, file_path)?;
        let file = File::new(*entry.name(), entry.addr());
        let node = self.data_allocator.allocate_node_data(&mut self.device, file_size)?;
        storage::store_data(&mut self.device, node.data_addrs(), data)?;
        storage::store(&mut self.device, file.node_addr(), &node)?;
        storage::store(&mut self.device, file.node_addr(), &file)?;
        Ok(())
    }

    pub fn delete(&mut self, file_path: &str) -> Result<(), Error> {
        paths::validate(file_path)?;

        let entry = Tree::get_file(&mut self.device, file_path)?;
        let node: Node = storage::load(&mut self.device, entry.addr())?;
        storage::erase::<_, Node>(&mut self.device, entry.addr())?;
        storage::erase::<_, File>(&mut self.device, entry.addr())?;
        Tree::remove_file(&mut self.device, file_path)?;
        Tree::prune(&mut self.device, &mut self.tree_allocator, 0)?;

        // Release data blocks only after metadata is fully erased.
        self.data_allocator.release_node_data(&mut self.device, &node)?;
        Ok(())
    }

    pub fn open(&mut self, file_path: &str) -> Result<DataReader<D>, Error> {
        paths::validate(file_path)?;

        let entry = Tree::get_file(&mut self.device, file_path)?;
        let node: Node = storage::load(&mut self.device, entry.addr())?;
        Ok(DataReader::new(&mut self.device, node))
    }

    pub fn count_files(&mut self) -> Result<usize, Error> {
        Tree::count_files(&mut self.device)
    }

    pub fn count_dirs(&mut self) -> Result<usize, Error> {
        Tree::count_dirs(&mut self.device)
    }

    pub fn count_free_data_blocks(&mut self) -> Result<usize, Error> {
        self.data_allocator.count_free_addresses(&mut self.device)
    }

    #[cfg(feature = "std")]
    pub fn print_tree(&mut self, base_path: &str, depth: usize) -> Result<(), Error> {
        use crate::filesystem::tree::printer;
        printer::print(&mut self.device, base_path, depth)
    }

    #[cfg(feature = "std")]
    pub fn print_disk_layout(&self) {
        use crate::filesystem::layouts;

        layouts::print();
    }
}
