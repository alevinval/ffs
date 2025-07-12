//! Filesystem represents file names as byte arrays with fixed capacity of
//! [`Name::MAX_LEN`], ensuring a fixed-size format when stored on disk.
//!
//! To facilitate type safety, incoming `&str` file naames are converted to
//! [`Name`] to ensure that file names are always valid and conform to the
//! maximum length constraint.

use crate::{
    Error,
    filesystem::{Deserializable, SerdeLen, Serializable, paths},
    io::{Read, Write},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Name {
    buf: [u8; Self::LEN],
    len: usize,
}

impl Name {
    /// Maximum length of a file name in bytes.
    pub const LEN: usize = 45;

    pub const fn empty() -> Self {
        Self { buf: [0u8; Self::LEN], len: 0 }
    }

    /// Creates a new [`FileName`] from a string slice.
    ///
    /// # Errors
    /// Returns an error if the provided name exceeds the maximum length of
    /// [`Self::MAX_LEN`].
    pub fn new(name: &str) -> Result<Self, Error> {
        assert!(
            !name.contains(paths::SEPARATOR),
            "File names should never contain a separator character"
        );

        if name.len() > Self::LEN {
            return Err(Error::FileNameTooLong);
        }

        let mut buf = [0u8; Self::LEN];
        let len = name.len();
        buf[..len].copy_from_slice(&name.as_bytes()[..len]);
        Ok(Self { buf, len })
    }

    /// Returns the file name as a string slice.
    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.buf[..self.len]).unwrap_or("<invalid utf8>")
    }
}

impl SerdeLen for Name {
    const SERDE_LEN: usize = Self::LEN + 1;
}

impl Serializable for Name {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = writer.write_u8(self.len as u8)?;
        n += writer.write(&self.buf)?;
        Ok(n)
    }
}

impl Deserializable<Self> for Name {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let len = reader.read_u8()? as usize;
        if len > Self::LEN {
            return Err(Error::FileNameTooLong);
        }

        let mut buffer = [0u8; Self::LEN];
        reader.read(&mut buffer)?;
        Ok(Self { buf: buffer, len })
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Name> for &Name {
    fn eq(&self, other: &Name) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        other.as_str() == *self
    }
}

#[cfg(test)]
impl From<&str> for Name {
    fn from(name: &str) -> Self {
        Self::new(name).expect("FileName::from should not fail with valid input")
    }
}

#[cfg(test)]
impl From<std::string::String> for Name {
    fn from(name: std::string::String) -> Self {
        Self::new(&name).expect("FileName::from should not fail with valid input")
    }
}

#[cfg(test)]
mod tests {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(Name, Name::new("test_file").unwrap());

    #[test]
    fn test_empty() {
        let sut = Name::empty();
        assert_eq!(sut.len, 0);
        assert_eq!(sut.buf, [0u8; Name::LEN]);
    }

    #[test]
    fn test_as_str() {
        let name = "valid_name";
        let sut = Name::new(name).unwrap();
        assert_eq!(name, sut.as_str());
    }

    #[test]
    fn test_name_exceeds_max_len() {
        let name = "b".repeat(Name::LEN + 1);
        let result = Name::new(&name);
        assert_eq!(Error::FileNameTooLong, result.unwrap_err());
    }
}
