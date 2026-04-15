use core::fmt;

use crate::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The provided buffer is too small for the expected data.
    BufferTooSmall { expected: usize, found: usize },
    /// The file already exists.
    FileAlreadyExists,
    /// The name exceeds the maximum allowed length.
    NameTooLong,
    /// The file does not exist.
    FileNotFound,
    /// The file is too large to be stored.
    FileTooLarge,
    /// The directory is not found.
    DirectoryNotFound,
    /// The directory is full and cannot accommodate more entries.
    DirectoryFull,
    /// The file system is full and cannot accommodate more files.
    StorageFull,
    /// The device is not formatted correctly.
    UnsupportedDevice,
    /// Unexpected
    Unexpected,
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        match value {
            io::Error::BufferTooSmall { expected, found } => {
                Self::BufferTooSmall { expected, found }
            }
            io::Error::IO { io: _ } => Self::UnsupportedDevice,
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(_: fmt::Error) -> Self {
        Self::Unexpected
    }
}
