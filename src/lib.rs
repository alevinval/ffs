#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[cfg(test)]
pub(crate) mod test_utils;

pub use filesystem::{BlockDevice, Controller};

use filesystem::{DirEntry, Name};

#[cfg(feature = "test-support")]
pub mod disk;

mod filesystem;
mod io;

pub struct Constants {}

impl Constants {
    pub const MAX_FILE_NAME_LEN: usize = Name::MAX_LEN;
    pub const MAX_EDGES: usize = DirEntry::MAX_EDGES;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The provided buffer is too small for the expected data.
    BufferTooSmall { expected: usize, found: usize },
    /// The file already exists.
    FileAlreadyExists,
    /// The file name exceeds the maximum allowed length.
    FileNameTooLong,
    /// The file does not exist.
    FileNotFound,
    /// The file is too large to be stored.
    FileTooLarge,
    /// The file name is invalid (e.g., contains invalid UTF-8).
    InvalidFileName,
    /// The file system is full and cannot accommodate more files.
    StorageFull,
    /// The operation is not supported by the current file system implementation.
    Unsupported,
}
