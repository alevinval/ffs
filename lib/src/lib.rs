#![no_std]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(any(test, feature = "test-support"))]
pub mod testutils;

pub use controller::Controller;
pub use error::Error;

use crate::{
    block::Block,
    io::{Read, Write},
    layouts::Layout,
    name::Name,
    tree::TreeNode,
};

mod allocator;
mod block;
mod cache;
pub mod constants;
mod controller;
mod data_reader;
mod error;
mod file;
mod io;
mod layouts;
mod meta;
mod name;
mod node;
mod paths;
mod storage;
mod tree;

pub type Addr = u32; // Logical address type for sectors/blocks. Change here to update everywhere.

/// Trait for types that have a constant length when serialized/deserialized.
pub trait FixedLen {
    /// Fixed length of the serialized type.
    const BYTES_LEN: usize;

    /// Number of blocks required to store the type.
    const BLOCKS_LEN: usize = Self::BYTES_LEN.div_ceil(Block::LEN);
}

pub trait Serializable: FixedLen {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error>;
}

pub trait Deserializable<T>: FixedLen
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
