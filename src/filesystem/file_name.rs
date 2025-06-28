//! Filesystem represents file names as byte arrays with fixed capacity of
//! [`MAX_FILENAME_LEN`], ensuring a fixed-size format when stored on disk.
//!
//! To facilitate type safety, incoming `&str` file naames are converted to
//! [`FileName`] to ensure that file names are always valid and conform to the
//! maximum length constraint.

use core::ops::Add;

use crate::{
    Error,
    filesystem::{Deserializable, MAX_FILENAME_LEN, SerdeLen, Serializable},
    io::{Read, Write, Writer},
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
        let name = canonicalize(name);

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

    /// Returns `true` if the file name is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the byte representation of the file name.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Returns the file name as a string slice.
    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.bytes[..self.len]).unwrap_or("<invalid utf8>")
    }

    /// Creates a new empty buffer to store a file name.
    const fn buffer() -> [u8; MAX_FILENAME_LEN] {
        [0u8; MAX_FILENAME_LEN]
    }

    /// Returns the directory name preceding the file name.
    pub fn dirname(&self) -> &str {
        self.as_str().rsplit_once('/').map(|(dirname, _)| dirname).unwrap_or_default()
    }

    /// Returns the file name without the directory part.
    pub fn basename(&self) -> &str {
        let str = self.as_str();
        str.rsplit_once('/').map(|(_, basename)| basename).unwrap_or(str)
    }

    /// Returns a new [`FileName`] that represents the inner path after the first component.
    pub fn tail(&self) -> Self {
        if self.basename().is_empty() {
            return *self;
        }

        let first = self.first_component();
        FileName::new(self.as_str().strip_prefix(first).unwrap()).unwrap()
    }

    /// Returns the first component of the file name, which is the part before the first slash.
    pub fn first_component(&self) -> &str {
        self.as_str().split('/').next().unwrap_or("")
    }
}

fn canonicalize(file_name: &str) -> &str {
    file_name.trim_start_matches('/').trim_end_matches('/')
}

impl Add for FileName {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        let new_len = self.len + other.len + 1;
        if new_len > MAX_FILENAME_LEN {
            panic!("filename addition exceeds maximum length");
        }

        let mut bytes = [0u8; MAX_FILENAME_LEN];
        let mut writer = Writer::new(&mut bytes);
        writer.write(self.as_bytes()).unwrap();
        writer.write_u8(b'/').unwrap();
        writer.write(other.as_bytes()).unwrap();

        Self { bytes, len: new_len }
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

impl PartialEq<str> for FileName {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for FileName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<FileName> for &FileName {
    fn eq(&self, other: &FileName) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<FileName> for &str {
    fn eq(&self, other: &FileName) -> bool {
        other.as_str() == *self
    }
}

#[cfg(test)]
impl From<&str> for FileName {
    fn from(name: &str) -> Self {
        Self::new(name).expect("FileName::from should not fail with valid input")
    }
}

#[cfg(test)]
impl From<std::string::String> for FileName {
    fn from(name: std::string::String) -> Self {
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

        assert_eq!(expected, actual.bytes);
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
    fn as_bytes_returns_slice() {
        let input = "abc";
        let sut = FileName::new(input).unwrap();
        let actual = sut.as_bytes();

        assert_eq!(input.as_bytes(), actual);
    }

    #[test]
    fn as_str_returns_valid_utf8() {
        let name = "valid_name";
        let sut = FileName::new(name).unwrap();
        assert_eq!(name, sut.as_str());
    }

    #[test]
    fn basename_and_dirname() {
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
    fn tail_path() {
        let input = FileName::new("foo/bar/baz").unwrap();
        let tail = input.tail();
        assert_eq!("bar/baz", tail.as_str());
        assert_eq!("baz", tail.tail().as_str());
    }

    #[test]
    fn addition() {
        let first = FileName::new("/foo").unwrap();
        let second = FileName::new("/bar/baz").unwrap();
        let addition = first + second;
        assert_eq!("foo/bar/baz", addition.as_str());
    }
}
