use crate::{
    BlockDevice, Error,
    filesystem::{
        EraseFrom, Layout, LoadFrom, LoadFromStatic, Store,
        allocator::{Allocator, DataAllocator},
        cache::BlockCache,
        data_writer::DataWriter,
        directory::{Directory, DirectoryNode},
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
    directory: Directory,
    allocator: Allocator,
}

impl<D> Controller<D>
where
    D: BlockDevice,
{
    pub fn mount(mut device: D) -> Result<Self, Error> {
        if Meta::load_from(&mut device)? != Meta::new() {
            return Err(Error::UnsupportedDevice);
        }
        let device = BlockCache::mount(device);
        let directory = Directory::new(Allocator::new(Layout::TREE_BITMAP));
        let allocator = Allocator::new(Layout::DATA_BITMAP);
        Ok(Self { device, directory, allocator })
    }

    pub fn unmount(self) -> D {
        self.device.unmount()
    }

    pub fn format(device: &mut D) -> Result<(), Error> {
        Meta::new().store(device)?;
        Allocator::new(Layout::TREE_BITMAP).allocate(device)?;
        DirectoryNode::new().store(device, 0)?;
        Ok(())
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

        let entry = self.directory.insert_file(&mut self.device, file_path)?;
        let file = File::new(*entry.name(), entry.addr());
        let node = self.allocator.allocate_node_data(&mut self.device, file_size)?;
        DataWriter::new(node.block_addrs(), data).store(&mut self.device)?;
        NodeWriter::new(file.node_addr(), &node).store(&mut self.device)?;
        file.store(&mut self.device)?;
        Ok(())
    }

    pub fn delete(&mut self, file_path: &str) -> Result<(), Error> {
        path::validate(file_path)?;

        let entry = self.directory.get_file(&mut self.device, file_path)?;
        let (file_handle, node_handle) = entry.get_handles();
        let node = node_handle.load_from(&mut self.device)?;

        node_handle.erase_from(&mut self.device)?;
        file_handle.erase_from(&mut self.device)?;
        self.directory.remove_file(&mut self.device, file_path)?;

        // Release data blocks only after metadata is fully erased.
        self.allocator.release_node_data(&mut self.device, &node)?;
        Ok(())
    }

    pub fn count_files(&mut self) -> Result<usize, Error> {
        self.directory.count_files(&mut self.device)
    }

    pub fn free_data_blocks(&mut self) -> Result<usize, Error> {
        self.allocator.count_free_addresses(&mut self.device)
    }

    #[cfg(feature = "std")]
    pub fn print_disk_layout(&self) {
        use std::println;

        println!("Disk layout:");
        println!("  Meta: {:?}", Layout::META);
        println!("  Tree bitmap: {:?}", Layout::TREE_BITMAP);
        println!("  Tree: {:?}", Layout::TREE);
        println!("  File: {:?}", Layout::FILE);
        println!("  Node: {:?}", Layout::NODE);
        println!("  Data bitmap: {:?}", Layout::DATA_BITMAP);
        println!("  Data: {:?}", Layout::DATA);
    }

    #[cfg(feature = "std")]
    pub fn print_tree(&mut self) -> Result<(), Error> {
        use crate::io::StdoutFmtWriter;

        self.directory.print_tree(&mut self.device, &mut StdoutFmtWriter)
    }
}
