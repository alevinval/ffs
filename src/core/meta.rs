use crate::{
    BLOCK_SIZE, Error, Index,
    serde::{Deserializable, Serializable},
    storage::Ranges,
};

#[derive(PartialEq, Eq, Debug)]
pub struct Meta {
    file_address: Index,
    node_address: Index,
    free_address: Index,
    data_address: Index,
    block_size: u16,
}

impl Meta {
    const SIGNATURE: [u8; 2] = [0x13, 0x37];

    pub const fn new(block_size: u16) -> Self {
        Meta {
            file_address: Ranges::FILE.begin(),
            node_address: Ranges::NODE.begin(),
            free_address: Ranges::FREE.begin(),
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
    fn serialize(&self, out: &mut [u8]) -> Result<usize, Error> {
        out[0..4].copy_from_slice(&self.file_address.to_le_bytes());
        out[4..8].copy_from_slice(&self.node_address.to_le_bytes());
        out[8..12].copy_from_slice(&self.free_address.to_le_bytes());
        out[12..16].copy_from_slice(&self.data_address.to_le_bytes());
        out[16..18].copy_from_slice(&self.block_size.to_le_bytes());
        out[510..512].copy_from_slice(&Self::SIGNATURE);
        Ok(BLOCK_SIZE)
    }
}

impl Deserializable<Meta> for Meta {
    fn deserialize(buf: &[u8]) -> Result<Meta, Error> {
        assert!(buf[510..512] == Self::SIGNATURE, "Invalid metadata block signature");

        Ok(Meta {
            file_address: Index::from_le_bytes(buf[0..4].try_into().unwrap()),
            node_address: Index::from_le_bytes(buf[4..8].try_into().unwrap()),
            free_address: Index::from_le_bytes(buf[8..12].try_into().unwrap()),
            data_address: Index::from_le_bytes(buf[12..16].try_into().unwrap()),
            block_size: u16::from_le_bytes(buf[16..18].try_into().unwrap()),
        })
    }
}

#[cfg(test)]
mod test {
    use crate::alloc_block_buffer;

    use super::*;

    #[test]
    fn serde_symmetry() -> Result<(), Error> {
        let mut buf = alloc_block_buffer();

        let expected = Meta::new(512);
        expected.serialize(&mut buf)?;
        let actual = Meta::deserialize(&buf)?;
        assert_eq!(expected, actual);

        Ok(())
    }
}
