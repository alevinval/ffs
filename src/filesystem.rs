pub use block::Block;
pub use controller::Controller;

use data_writer::DataWriter;
use file::File;
use file_name::FileName;
use free_allocator::FreeBlockAllocator;
use meta::Meta;
use node::Node;
use node_writer::NodeWriter;
use range::Range;

use crate::{Error, io::Write};

mod block;
mod controller;
mod data_writer;
mod file;
mod file_handle;
mod file_name;
mod free;
mod free_allocator;
mod meta;
mod node;
mod node_writer;
mod range;

pub type Addr = u32; // Logical address type for sectors/blocks. Change here to update everywhere.

const MAX_FILENAME_LEN: usize = 128; // Maximum file name length
const MAX_FILES: usize = 1024; // Maximum number of files in the file system
const MAX_DATA_BLOCKS: usize = Node::BLOCKS * MAX_FILES;

trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), Error>;
}

trait Deserializable<T>
where
    T: Sized,
{
    fn deserialize(buf: &[u8]) -> Result<T, Error>;
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
        assert!(a.end() == b.begin(), "range {:?} does not end where {:?} begins", a, b);
    }

    #[test]
    fn ranges_layout() {
        assert_continuous_range(Meta::RANGE, File::RANGE);
        assert_continuous_range(File::RANGE, Node::RANGE);
        assert_continuous_range(Node::RANGE, FreeBlockAllocator::RANGE);
        assert_continuous_range(FreeBlockAllocator::RANGE, DataWriter::RANGE);
    }
}
