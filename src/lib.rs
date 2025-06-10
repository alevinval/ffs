use storage::reader::Reader;

pub use core::{Data, File, Free, Meta, Node};

use std::io;

use crate::storage::writer::Writer;

mod core;
pub mod disk;
pub mod serde;
pub mod storage;

pub const BLOCK_SIZE: usize = 512;
pub const MAX_BLOCKS_PER_NODE: usize = 10; // Maximum number of blocks per file
pub const MAX_DATA_BLOCKS: usize = MAX_BLOCKS_PER_NODE * MAX_FILES as usize;
pub const MAX_FILENAME_LENGTH: usize = 128; // Maximum file name length
pub const MAX_FILES: u32 = 1024; // Maximum number of files in the file system
pub const MAX_FILESYSTEM_BLOCKS: usize = 16 * 1024; // Maximum number of blocks in the file system

pub const DATA_BLOCKS_COUNT: usize = MAX_DATA_BLOCKS / Free::CAPACITY;

pub type Index = u32;

pub const fn alloc_block_buffer() -> [u8; BLOCK_SIZE] {
    [0u8; BLOCK_SIZE]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidMetadata,
    FileNotFound,
    FailedIO,
}

#[derive(Debug)]
pub struct Table {
    metadata: Meta,
    files: [Option<File>; MAX_FILES as usize],
    nodes: [Option<Node>; MAX_FILES as usize],
    free_blocks: [Free; DATA_BLOCKS_COUNT],
    size: u32,
}

impl Table {
    pub fn from<T>(input: &mut T) -> Result<Table, Error>
    where
        T: io::Seek + io::Read,
    {
        let metadata = Reader::read_metadata(input).map_err(|_| Error::InvalidMetadata)?;

        println!("[ffs] Metadata: {:?}", metadata);
        let mut table = Table::new();
        table.metadata = metadata;

        Ok(table)
    }

    pub fn new() -> Self {
        Table {
            metadata: Meta::new(BLOCK_SIZE as u16),
            files: [const { None }; MAX_FILES as usize],
            nodes: [const { None }; MAX_FILES as usize],
            free_blocks: [const { Free::new() }; DATA_BLOCKS_COUNT],
            size: 0,
        }
    }

    pub fn create<T>(
        &mut self,
        filename: &str,
        file_size: u16,
        data: &[u8],
        mut out: &mut T,
    ) -> (&File, &Node)
    where
        T: io::Seek + io::Write + io::Read,
    {
        assert!(
            file_size as usize <= MAX_BLOCKS_PER_NODE * BLOCK_SIZE,
            "File size exceeds maximum filesystem blocks"
        );
        assert!(filename.len() <= MAX_FILENAME_LENGTH, "Filename exceeds maximum length");
        assert!(self.size < self.files.len() as u32, "Maximum number of files reached");

        let blocks_needed = (file_size as usize).div_ceil(BLOCK_SIZE);
        let mut block_indexes = [0; MAX_BLOCKS_PER_NODE];
        for block_index in block_indexes.iter_mut().take(blocks_needed) {
            *block_index = self.find_free_block().expect("no space left");
        }

        let node = Node::new(file_size, block_indexes);
        let file = File::new(filename, self.size);

        Writer::File(&file).write(&mut out).expect("cannot write file");
        Writer::Node(&file, &node).write(&mut out).expect("cannot write node");
        Writer::write_chunks(&node, data, &mut out).expect("cannot write chunks");

        let pos = self.size as usize;
        self.nodes[pos] = Some(node);
        self.files[pos] = Some(file);
        self.size += 1;

        (self.files[pos].as_ref().expect("asd"), self.nodes[pos].as_ref().expect("asd"))
    }

    pub fn delete<T>(&mut self, filename: &str, mut disk: &mut T) -> Result<(), Error>
    where
        T: io::Seek + io::Write + io::Read,
    {
        if let Some(file) = self.find_file(filename) {
            let idx = file.get_node_index() as usize;
            let node = self.nodes[idx].as_ref().expect("should be there");
            Writer::Node(file, node).erase(&mut disk).map_err(|_| Error::FailedIO)?;
            Writer::File(file).erase(&mut disk).map_err(|_| Error::FailedIO)?;

            self.nodes[idx] = None;
            self.files[idx] = None;
            return Ok(());
        }

        Err(Error::FileNotFound)
    }

    pub fn print_ls(&self) {
        for file in self.files.iter().flatten() {
            println!("- {}", file.get_name())
        }
    }

    fn find_file(&self, name: &str) -> Option<&File> {
        self.files.iter().flatten().find(|f| f.get_name() == name)
    }

    fn find_free_block(&mut self) -> Option<Index> {
        self.free_blocks
            .iter_mut()
            .enumerate()
            .map(|(idx, f)| (idx, f.take_free_block()))
            .find(|(_, f)| f.is_some())
            .and_then(|(idx, f)| f.map(|a| a + (idx * Free::CAPACITY) as u32))
    }
}

#[cfg(test)]
mod test {}
