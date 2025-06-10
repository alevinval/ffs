use core::str;

use crate::{
    Error, Index, MAX_FILENAME_LENGTH,
    serde::{Deserializable, Serializable},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct File {
    name: [u8; MAX_FILENAME_LENGTH],
    name_length: u8,
    node_index: Index,
}

impl File {
    pub fn from_str(name: &str, node_index: Index) -> Result<Self, Error> {
        if name.len() > MAX_FILENAME_LENGTH {
            return Err(Error::FileNameTooLong);
        }
        let mut fname = [0u8; MAX_FILENAME_LENGTH];
        fname[0..name.len()].copy_from_slice(name.as_bytes());
        Ok(Self::new(fname, name.len() as u8, node_index))
    }

    pub const fn new(name: [u8; MAX_FILENAME_LENGTH], name_length: u8, node_index: Index) -> Self {
        File { name, name_length, node_index }
    }

    pub fn get_name(&self) -> &str {
        let slice = &self.name[..self.name_length as usize];
        str::from_utf8(slice).unwrap_or("<invalid utf8>")
    }

    pub const fn get_name_length(&self) -> u8 {
        self.name_length
    }

    pub const fn get_node_index(&self) -> Index {
        self.node_index
    }
}

impl Serializable for File {
    fn serialize(&self, buf: &mut [u8]) -> Result<usize, Error> {
        buf[0] = self.name_length;
        buf[1..5].copy_from_slice(&self.node_index.to_le_bytes());
        buf[5..5 + MAX_FILENAME_LENGTH].copy_from_slice(&self.name);
        Ok(1 + 4 + MAX_FILENAME_LENGTH)
    }
}

impl Deserializable<File> for File {
    fn deserialize(buf: &[u8]) -> Result<File, Error> {
        let mut node_index = [0u8; 4];
        node_index.copy_from_slice(&buf[1..5]);

        let mut name = [0u8; MAX_FILENAME_LENGTH];
        name.copy_from_slice(&buf[5..5 + MAX_FILENAME_LENGTH]);

        Ok(File { name, name_length: buf[0], node_index: u32::from_le_bytes(node_index) })
    }
}

#[cfg(test)]
mod test {
    use crate::alloc_block_buffer;

    use super::*;

    #[test]
    fn getters() {
        let sut = File::from_str("test.txt", 123).unwrap();

        assert_eq!(8, sut.get_name_length());
        assert_eq!("test.txt", sut.get_name());
        assert_eq!(123, sut.get_node_index())
    }

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut buf = alloc_block_buffer();

        let expected = File::from_str("test.txt", 123).unwrap();
        expected.serialize(&mut buf)?;
        let actual = File::deserialize(&buf)?;

        assert_eq!(expected, actual);
        Ok(())
    }
}
