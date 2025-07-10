pub use controller::Controller;
pub use data_reader::DataReader;
pub use directory::TreeNode;
pub use name::Name;
pub use node::Node;

use crate::{
    Error,
    io::{Read, Write},
};
use block::Block;
use layout::Layout;

pub mod allocator;
mod block;
mod cache;
mod controller;
mod data_reader;
mod directory;
mod file;
mod layout;
mod meta;
mod name;
mod node;
mod paths;
mod storage;

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
    fn read(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error>;

    fn write(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error>;
}

pub trait Addressable {
    const LAYOUT: Layout;
}

pub trait LoadFrom<D>
where
    D: BlockDevice,
{
    type Item: Sized;

    fn load_from(device: &mut D, logical_addr: Addr) -> Result<Self::Item, Error>;
}

pub trait EraseFrom<D>
where
    D: BlockDevice,
{
    fn erase_from(device: &mut D, logical_addr: Addr) -> Result<(), Error>;
}

impl<T, D: BlockDevice> LoadFrom<D> for T
where
    T: Addressable + Deserializable<T>,
{
    type Item = T;

    fn load_from(device: &mut D, logical_addr: Addr) -> Result<Self::Item, Error> {
        let sector = T::LAYOUT.nth(logical_addr);
        let mut block = Block::new();
        device.read(sector, &mut block)?;
        T::deserialize(&mut block.reader())
    }
}

impl<T, D: BlockDevice> EraseFrom<D> for T
where
    T: Addressable,
{
    fn erase_from(device: &mut D, logical_addr: Addr) -> Result<(), Error> {
        let sector = T::LAYOUT.nth(logical_addr);
        device.write(sector, &Block::new())
    }
}
