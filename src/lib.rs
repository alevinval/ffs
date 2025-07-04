#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(test)]
pub(crate) mod test_utils;

pub use filesystem::{BlockDevice, Controller};

use crate::filesystem::{DirectoryNode, Name};

#[cfg(feature = "test-support")]
pub mod disk;

mod filesystem;
mod io;

pub struct Constants {}

impl Constants {
    pub const FILENAME_LEN: usize = Name::LEN;
    pub const DIR_NODE_ENTRIES: usize = DirectoryNode::LEN;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The provided buffer is too small for the expected data.
    BufferTooSmall { expected: usize, found: usize },
    /// The file already exists.
    FileAlreadyExists,
    /// The name exceeds the maximum allowed length.
    FileNameTooLong,
    /// The file does not exist.
    FileNotFound,
    /// The file is too large to be stored.
    FileTooLarge,
    /// The file system is full and cannot accommodate more files.
    StorageFull,
    /// The device is not formatted correctly.
    UnsupportedDevice,
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        match value {
            io::Error::BufferTooSmall { expected, found } => {
                Self::BufferTooSmall { expected, found }
            }
        }
    }
}
