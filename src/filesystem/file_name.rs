//! Filesystem represents file names as byte arrays with fixed capacity of
//! [`MAX_FILENAME_LEN`], ensuring a fixed-size format when stored on disk.
//!
//! To facilitate type safety, incoming `&str` file naames are converted to
//! [`FileName`] to ensure that file names are always valid and conform to the
//! maximum length constraint.

use core::ops::Add;
#[cfg(test)]
use std::string::String;

use crate::{
    Error,
    filesystem::{Deserializable, MAX_FILENAME_LEN, SerdeLen, Serializable},
    io::{Read, Write},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FileName {
    bytes: [u8; MAX_FILENAME_LEN],
    len: usize,
}

impl FileName {
    pub const fn empty() -> Self {
        Self { bytes: Self::buffer(), len: 0 }
    }

    /// Creates a new [`FileName`] from a string slice.
    ///
    /// # Errors
    /// Returns an error if the provided name exceeds the maximum length of
    /// [`MAX_FILENAME_LEN`].
    pub fn new(name: &str) -> Result<Self, Error> {
        let name = name.trim_start_matches('/').trim_end_matches('/');

        if name.len() > MAX_FILENAME_LEN {
            return Err(Error::FileNameTooLong);
        }

        let mut bytes = Self::buffer();
        let len = name.len().min(MAX_FILENAME_LEN);
        bytes[..len].copy_from_slice(&name.as_bytes()[..len]);
        Ok(Self { bytes, len })
    }

    /// Returns the length of the file name.
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the byte representation of the file name.
    pub const fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.bytes[..self.len]).unwrap_or("<invalid utf8>")
    }

    /// Creates a new empty buffer to store a file name.
    const fn buffer() -> [u8; MAX_FILENAME_LEN] {
        [0u8; MAX_FILENAME_LEN]
    }

    pub fn dirname(&self) -> &str {
        let str = self.as_str();
        if let Some((base_name, _)) = str.rsplit_once('/') {
            return base_name;
        }
        ""
    }

    pub fn basename(&self) -> &str {
        let str = self.as_str();
        if let Some((_, dir_name)) = str.rsplit_once('/') {
            return dir_name;
        }
        str
    }

    pub fn inside(&self) -> Self {
        let basename = self.basename();
        if basename.is_empty() {
            return *self;
        }

        let str = self.as_str();
        let first = first_component(str);
        FileName::new(str.strip_prefix(first).unwrap_or(str)).unwrap()
    }
}

fn first_component(path: &str) -> &str {
    path.trim_start_matches('/').split('/').next().unwrap_or("")
}

impl Add for FileName {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        if self.len + other.len > MAX_FILENAME_LEN {
            panic!("filename addition exceeds maximum filename length");
        }

        let mut bytes = [0u8; MAX_FILENAME_LEN];
        let len = self.len + other.len;
        bytes[..self.len].copy_from_slice(&self.bytes[..self.len]);
        bytes[self.len..len].copy_from_slice(&other.bytes[..other.len]);

        Self { bytes, len }
    }
}

impl SerdeLen for FileName {
    const SERDE_LEN: usize = MAX_FILENAME_LEN + 1;
}

impl Serializable for FileName {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u8(self.len as u8)?;
        n += writer.write(&self.bytes)?;
        Ok(n)
    }
}

impl Deserializable<FileName> for FileName {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let len = reader.read_u8()? as usize;
        if len > MAX_FILENAME_LEN {
            return Err(Error::FileNameTooLong);
        }

        let mut bytes = [0u8; MAX_FILENAME_LEN];
        reader.read(&mut bytes)?;
        Ok(Self { bytes, len })
    }
}

#[cfg(test)]
impl From<&str> for FileName {
    fn from(name: &str) -> Self {
        Self::new(name).expect("FileName::from should not fail with valid input")
    }
}

#[cfg(test)]
impl From<String> for FileName {
    fn from(name: String) -> Self {
        Self::new(&name).expect("FileName::from should not fail with valid input")
    }
}

#[cfg(test)]
mod tests {
    use crate::filesystem::Block;

    use super::*;

    #[test]
    fn serde_symmetry() {
        let mut block = Block::new();

        let expected = FileName::new("test_file").unwrap();
        assert_eq!(Ok(FileName::SERDE_LEN), expected.serialize(&mut block.writer()));
        let actual = FileName::deserialize(&mut block.reader()).unwrap();

        assert_eq!(expected, actual);
    }

    #[test]
    fn short_length() {
        let name = "testfile";
        let actual = FileName::new(name).unwrap();

        let mut expected = [0u8; MAX_FILENAME_LEN];
        expected[..name.len()].copy_from_slice(name.as_bytes());

        assert_eq!(expected.as_ref(), actual.as_bytes());
    }

    #[test]
    fn exact_length() {
        let name = "a".repeat(MAX_FILENAME_LEN);
        let actual = FileName::new(&name).unwrap();
        let expected = name.as_bytes();
        assert_eq!(expected, &actual.as_bytes()[..MAX_FILENAME_LEN]);
    }

    #[test]
    fn name_too_long() {
        let name = "b".repeat(MAX_FILENAME_LEN + 1);
        let result = FileName::new(&name);
        assert_eq!(Err(Error::FileNameTooLong), result);
    }

    #[test]
    fn as_bytes_returns_full_array() {
        let input = "abc";
        let sut = FileName::new(input).unwrap();
        let actual = sut.as_bytes();

        assert_eq!(MAX_FILENAME_LEN, actual.len());
        assert_eq!(input.as_bytes(), &actual[..input.len()]);
        assert!(actual[input.len()..].iter().all(|&b| b == 0));
    }

    #[test]
    fn as_str_returns_valid_utf8() {
        let name = "valid_name";
        let sut = FileName::new(name).unwrap();
        assert_eq!(name, sut.as_str());
    }

    #[test]
    fn base_name_and_dir_name() {
        let name = FileName::new("/path/to/file.txt").unwrap();
        assert_eq!("path/to", name.dirname());
        assert_eq!("file.txt", name.basename());

        let name = FileName::new("file.txt").unwrap();
        assert_eq!("", name.dirname());
        assert_eq!("file.txt", name.basename());

        let name = FileName::new("/").unwrap();
        assert_eq!("", name.dirname());
        assert_eq!("", name.basename());

        let name = FileName::new("").unwrap();
        assert_eq!("", name.dirname());
        assert_eq!("", name.basename());
    }

    #[test]
    fn inside_returns_correct_file_name() {
        let name = FileName::new("foo/bar/baz").unwrap();
        let inside = name.inside();
        assert_eq!("bar/baz", inside.as_str());
    }
}
