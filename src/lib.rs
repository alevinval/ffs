use storage::reader::Reader;

pub use core::{Data, File, Free, Meta, Node};
pub use error::Error;

use crate::storage::writer::Writer;

mod core;
pub mod disk;
mod error;
pub mod serde;
pub mod storage;

pub const BLOCK_SIZE: usize = 512;
pub const MAX_BLOCKS_PER_NODE: usize = 10; // Maximum number of blocks per file
pub const MAX_DATA_BLOCKS: usize = MAX_BLOCKS_PER_NODE * MAX_FILES;
pub const MAX_FILENAME_LENGTH: usize = 128; // Maximum file name length
pub const MAX_FILES: usize = 1024; // Maximum number of files in the file system

pub const FREE_BLOCKS_COUNT: usize = MAX_DATA_BLOCKS / Free::SLOTS_COUNT;

pub type Index = u32;

pub const fn alloc_block_buffer() -> [u8; BLOCK_SIZE] {
    [0u8; BLOCK_SIZE]
}

pub trait BlockDevice {
    fn read_block(&mut self, sector: Index, buf: &mut [u8]) -> Result<(), Error>;
    fn write_block(&mut self, sector: Index, buf: &[u8]) -> Result<(), Error>;
}

#[derive(Debug)]
pub struct Controller {
    meta: Meta,
    file: [Option<File>; MAX_FILES],
    node: [Option<Node>; MAX_FILES],
    free: [Free; FREE_BLOCKS_COUNT],
    size: Index,
}

impl Controller {
    pub fn from<D>(device: &mut D) -> Result<Controller, Error>
    where
        D: BlockDevice,
    {
        let meta = Reader::read_metadata(device).map_err(|_| Error::InvalidMetadata)?;
        Ok(Controller::from_meta(meta))
    }

    pub const fn from_meta(meta: Meta) -> Self {
        Controller {
            meta,
            file: [const { None }; MAX_FILES],
            node: [const { None }; MAX_FILES],
            free: [const { Free::new() }; FREE_BLOCKS_COUNT],
            size: 0,
        }
    }

    pub fn create<D>(
        &mut self,
        filename: &str,
        file_size: u16,
        data: &[u8],
        out: &mut D,
    ) -> (&File, &Node)
    where
        D: BlockDevice,
    {
        assert!(
            file_size as usize <= MAX_BLOCKS_PER_NODE * BLOCK_SIZE,
            "File size exceeds maximum filesystem blocks"
        );
        assert!(filename.len() <= MAX_FILENAME_LENGTH, "Filename exceeds maximum length");
        assert!((self.size as usize) < self.file.len(), "Maximum number of files reached");

        let blocks_needed = (file_size as usize).div_ceil(BLOCK_SIZE);
        let mut block_indexes = [0; MAX_BLOCKS_PER_NODE];
        for block_index in block_indexes.iter_mut().take(blocks_needed) {
            *block_index = self.find_free_block().expect("no space left");
        }

        let node = Node::new(file_size, block_indexes);
        let file = File::from_str(filename, self.size).expect("file");

        Writer::File(&file).write(out).expect("cannot write file");
        Writer::Node(&file, &node).write(out).expect("cannot write node");
        Writer::write_chunks(&node, data, out).expect("cannot write chunks");

        let pos = self.size as usize;
        self.node[pos] = Some(node);
        self.file[pos] = Some(file);
        self.size += 1;

        (self.file[pos].as_ref().expect("asd"), self.node[pos].as_ref().expect("asd"))
    }

    pub fn delete<D>(&mut self, filename: &str, disk: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        if let Some(file) = self.find_file(filename) {
            let idx = file.get_node_index() as usize;
            let node = self.node[idx].as_ref().expect("should be there");
            Writer::Node(file, node).erase(disk).map_err(|_| Error::FailedIO)?;
            Writer::File(file).erase(disk).map_err(|_| Error::FailedIO)?;

            self.node[idx] = None;
            self.file[idx] = None;
            self.size -= 1;

            return Ok(());
        }

        Err(Error::FileNotFound)
    }

    pub fn print_ls(&self) {
        for file in self.file.iter().flatten() {
            println!("- {}", file.get_name())
        }
    }

    fn find_file(&self, name: &str) -> Option<&File> {
        self.file.iter().flatten().find(|f| f.get_name() == name)
    }

    fn find_free_block(&mut self) -> Option<Index> {
        self.free
            .iter_mut()
            .enumerate()
            .map(|(idx, f)| (idx, f.take_free_block()))
            .find(|(_, f)| f.is_some())
            .and_then(|(idx, f)| f.map(|a| a + (idx * Free::SLOTS_COUNT) as Index))
    }
}

#[cfg(test)]
mod test {}
