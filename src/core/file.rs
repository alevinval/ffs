use core::str;
use std::io::{self, Cursor, Read, Write};

use crate::{
    Index, MAX_FILENAME_LENGTH,
    serde::{Deserializable, Serializable},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct File {
    name: [u8; MAX_FILENAME_LENGTH],
    name_length: u8,
    node_index: Index,
}

impl File {
    pub fn new(name: &str, node_index: u32) -> Self {
        assert!(name.len() <= MAX_FILENAME_LENGTH, "File name exceeds maximum length");

        let name_length = name.len().min(MAX_FILENAME_LENGTH);
        let name_bytes = &name.as_bytes()[0..name_length];

        let mut name = [0; MAX_FILENAME_LENGTH];
        name[..name_length].copy_from_slice(name_bytes);
        File { name, name_length: name_length as u8, node_index }
    }

    pub fn get_name(&self) -> &str {
        let slice = &self.name[..self.name_length as usize];
        str::from_utf8(slice).expect("invalid UTF-8 in file name")
    }

    pub const fn get_name_length(&self) -> u8 {
        self.name_length
    }

    pub const fn get_node_index(&self) -> u32 {
        self.node_index
    }
}

impl Serializable for File {
    fn serialize(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut cursor = Cursor::new(buf);
        let mut n = cursor.write(&self.name_length.to_le_bytes())?;
        n += cursor.write(&self.node_index.to_le_bytes())?;
        n += cursor.write(&self.name)?;
        Ok(n)
    }
}

impl Deserializable<File> for File {
    fn deserialize(buf: &[u8]) -> io::Result<File> {
        let mut cursor = Cursor::new(buf);

        let mut name_len_buf = [0u8; 1];
        cursor.read_exact(&mut name_len_buf)?;

        let mut node_index_buf = [0u8; 4];
        cursor.read_exact(&mut node_index_buf)?;

        let mut name_buf = [0u8; MAX_FILENAME_LENGTH];
        cursor.read_exact(&mut name_buf)?;

        Ok(File {
            name: name_buf,
            name_length: u8::from_le_bytes(name_len_buf),
            node_index: Index::from_le_bytes(node_index_buf),
        })
    }
}

#[cfg(test)]
mod test {
    use crate::alloc_block_buffer;

    use super::*;

    #[test]
    fn getters() {
        let sut = File::new("test.txt", 123);

        assert_eq!(8, sut.get_name_length());
        assert_eq!("test.txt", sut.get_name());
        assert_eq!(123, sut.get_node_index())
    }

    #[test]
    fn serde_symmetry() -> io::Result<()> {
        let mut buf = alloc_block_buffer();

        let expected = File::new("test.txt", 123);
        expected.serialize(&mut buf)?;
        let actual = File::deserialize(&buf)?;

        assert_eq!(expected, actual);
        Ok(())
    }
}
