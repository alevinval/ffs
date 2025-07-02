pub use controller::Controller;
pub use directory::DirectoryNode;
pub use name::Name;

use crate::{
    Error,
    io::{Read, Write},
};
use block::Block;
use free::Free;
use layout::Layout;
use node::Node;

mod block;
mod cache;
mod controller;
mod data_allocator;
mod data_writer;
mod directory;
mod file;
mod free;
mod handle;
mod layout;
mod meta;
mod name;
mod node;
mod node_writer;
mod path;

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
    fn read_block(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error>;

    fn write_block(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error>;
}

pub trait Addressable {
    fn layout() -> Layout;
}

pub trait Store<D>
where
    D: BlockDevice,
{
    fn store(&self, device: &mut D) -> Result<(), Error>;
}

pub trait LoadFromStatic<D>
where
    D: BlockDevice,
{
    type Item: Sized;

    fn load_from(device: &mut D) -> Result<Self::Item, Error>;
}

pub trait LoadFrom<D>
where
    D: BlockDevice,
{
    type Item: Sized;

    fn load_from(&self, device: &mut D) -> Result<Self::Item, Error>;
}

pub trait EraseFrom<D>
where
    D: BlockDevice,
{
    fn erase_from(&self, device: &mut D) -> Result<(), Error>;
}
