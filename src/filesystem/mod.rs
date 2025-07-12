pub use controller::Controller;
pub use data_reader::DataReader;
pub use name::Name;
pub use node::Node;
pub use tree::TreeNode;

use crate::{
    Error,
    io::{Read, Write},
};
use block::Block;
use layouts::Layout;

pub mod allocator;
mod block;
mod cache;
mod controller;
mod data_reader;
mod file;
mod layouts;
mod meta;
mod name;
mod node;
mod paths;
mod storage;
mod tree;

pub type Addr = u32; // Logical address type for sectors/blocks. Change here to update everywhere.

/// Trait for types that have a constant length when serialized/deserialized.
trait SerdeLen {
    const SERDE_LEN: usize;
    const SERDE_BLOCK_COUNT: usize = Self::SERDE_LEN.div_ceil(Block::LEN);
    const SERDE_BUFFER_LEN: usize = Self::SERDE_BLOCK_COUNT * Block::LEN;
}

pub trait Serializable {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error>;
}

pub trait Deserializable<T>
where
    T: Sized,
{
    fn deserialize<R: Read>(reader: &mut R) -> Result<T, Error>;
}

pub trait BlockDevice {
    /// Reads a block of data from the specified sector into the provided buffer.
    fn read(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error>;

    /// Writes a block of data to the specified sector.
    fn write(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error>;
}

pub trait Addressable {
    const LAYOUT: Layout;
}
