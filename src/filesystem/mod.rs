pub use controller::Controller;
pub use directory::DirEntry;

use crate::{
    Error,
    io::{Read, Write},
};

use block::Block;
use cache::BlockCache;
use data_allocator::DataAllocator;
use data_writer::DataWriter;
use directory::Directory;
use file::File;
use file_handle::FileHandle;
use file_name::FileName;
use free::Free;
use meta::Meta;
use node::Node;
use node_handle::NodeHandle;
use node_writer::NodeWriter;
use range::Range;

mod block;
mod cache;
mod controller;
mod data_allocator;
mod data_writer;
mod directory;
mod file;
mod file_handle;
mod file_name;
mod free;
mod meta;
mod node;
mod node_handle;
mod node_writer;
pub mod path;
mod range;

pub type Addr = u32; // Logical address type for sectors/blocks. Change here to update everywhere.

/// Maximum number of entries in the B-tree used for directory entries.
const MAX_BTREE_ENTRIES: usize = 50;

/// Maximum number of files in the file system
const MAX_FILES: usize = MAX_BTREE_ENTRIES * DirEntry::MAX_CHILD_FILES;

/// Maximum number of data blocks in the file system.
const MAX_DATA_BLOCKS: usize = Node::BLOCKS_PER_NODE * MAX_FILES;

/// Maximum length of a file name in bytes.
const MAX_FILENAME_LEN: usize = 63;

pub struct Layout {}

/// Layout of the file system in the block device. Used to apply the required
/// offsets to logical addresses.
impl Layout {
    pub const META: Range = Range::new(0, 1);
    pub const BTREE: Range = Self::META.next_range(MAX_BTREE_ENTRIES, DirEntry::SERDE_BLOCK_COUNT);
    pub const FILE: Range = Self::BTREE.next_range(MAX_FILES, 1);
    pub const NODE: Range = Self::FILE.next_range(MAX_FILES, 1);
    pub const FREE: Range = Self::NODE.next_range(MAX_DATA_BLOCKS / Free::SLOTS, 1);
    pub const DATA: Range = Self::FREE.next_range(MAX_DATA_BLOCKS, 1);
}

/// Trait for types that have a constant length when serialized/deserialized.
trait SerdeLen {
    const SERDE_LEN: usize;
    const SERDE_BLOCK_COUNT: usize = Self::SERDE_LEN.div_ceil(Block::LEN);
    const SERDE_BUFFER_LEN: usize = Self::SERDE_BLOCK_COUNT * Block::LEN;
}

trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error>;
}

trait Deserializable<T>
where
    T: Sized,
{
    fn deserialize<R: Read>(reader: &mut R) -> Result<T, Error>;
}

pub trait BlockDevice {
    fn read_block(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error>;

    fn write_block(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error>;
}

trait WriteToDevice<D>
where
    D: BlockDevice,
{
    fn write_to_device(&self, device: &mut D) -> Result<(), Error>;
}

trait StaticReadFromDevice<D>
where
    D: BlockDevice,
{
    type Item: Sized;

    fn read_from_device(device: &mut D) -> Result<Self::Item, Error>;
}

trait ReadFromDevice<D>
where
    D: BlockDevice,
{
    type Item: Sized;

    fn read_from_device(&self, device: &mut D) -> Result<Self::Item, Error>;
}

trait EraseFromDevice<D>
where
    D: BlockDevice,
{
    fn erase_from_device(&self, device: &mut D) -> Result<(), Error>;
}

#[cfg(test)]
mod test {

    use super::*;

    fn assert_continuous_range(a: Range, b: Range) {
        assert!(a.end == b.begin, "range {a:?} does not end where {b:?} begins");
    }

    #[test]
    fn ranges_layout() {
        assert_continuous_range(Layout::META, Layout::BTREE);
        assert_continuous_range(Layout::BTREE, Layout::FILE);
        assert_continuous_range(Layout::FILE, Layout::NODE);
        assert_continuous_range(Layout::NODE, Layout::FREE);
        assert_continuous_range(Layout::FREE, Layout::DATA);
    }
}
