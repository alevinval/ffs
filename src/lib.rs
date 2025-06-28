#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub(crate) use filesystem::Addr;

pub use filesystem::{BlockDevice, Controller, DirEntry};

pub mod disk;
mod filesystem;
pub mod io;

#[cfg(test)]
pub(crate) mod test_utils;

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
