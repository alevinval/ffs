use std::io::{self, Cursor, Write};

use crate::{
    BLOCK_SIZE, Index,
    serde::{Deserializable, Serializable},
    storage::Ranges,
};

#[derive(PartialEq, Eq, Debug)]
pub struct Meta {
    file_address: Index,
    node_address: Index,
    data_address: Index,
    block_size: u16,
}

impl Meta {
    const SIGNATURE: [u8; 2] = [0x13, 0x37];

    pub fn new(block_size: u16) -> Self {
        Meta {
            file_address: Ranges::FILE.begin(),
            node_address: Ranges::NODE.begin(),
            data_address: Ranges::DATA.begin(),
            block_size,
        }
    }

    pub const fn get_file_address(&self) -> Index {
        self.file_address
    }

    pub const fn get_node_address(&self) -> Index {
        self.node_address
    }

    pub const fn get_data_address(&self) -> Index {
        self.data_address
    }

    pub const fn get_block_size(&self) -> u16 {
        self.block_size
    }
}

impl Serializable for Meta {
    fn serialize(&self, out: &mut [u8]) -> io::Result<usize> {
        let mut cursor = Cursor::new(out);
        let mut n = cursor.write(&self.file_address.to_le_bytes())?;
        n += cursor.write(&self.node_address.to_le_bytes())?;
        n += cursor.write(&self.data_address.to_le_bytes())?;
        n += cursor.write(&self.block_size.to_le_bytes())?;
        cursor.set_position(510);
        n += cursor.write(&Self::SIGNATURE)?;

        debug_assert!(n < BLOCK_SIZE);
        Ok(BLOCK_SIZE)
    }
}

impl Deserializable<Meta> for Meta {
    fn deserialize(buf: &[u8]) -> std::io::Result<Meta> {
        assert!(
            buf[BLOCK_SIZE - 2..BLOCK_SIZE] == Self::SIGNATURE,
            "Invalid metadata block signature"
        );

        Ok(Meta {
            file_address: Index::from_le_bytes(buf[0..4].try_into().unwrap()),
            node_address: Index::from_le_bytes(buf[4..8].try_into().unwrap()),
            data_address: Index::from_le_bytes(buf[8..12].try_into().unwrap()),
            block_size: u16::from_le_bytes(buf[12..14].try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod test {
    use crate::alloc_block_buffer;

    use super::*;

    #[test]
    fn serde_symmetry() -> io::Result<()> {
        let mut buf = alloc_block_buffer();

        let expected = Meta::new(512);
        expected.serialize(&mut buf)?;
        let actual = Meta::deserialize(&buf)?;
        assert_eq!(expected, actual);

        Ok(())
    }
}
