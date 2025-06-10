use crate::{
    BLOCK_SIZE, Error, Index,
    serde::{Deserializable, Serializable},
};

// Represents/tracks free
#[derive(PartialEq, Eq, Debug)]
pub struct Free {
    bitmap: [u8; BLOCK_SIZE],
    last_free: usize,
}

impl Default for Free {
    fn default() -> Self {
        Self::new()
    }
}

impl Free {
    pub const SLOTS_COUNT: usize = 8 * BLOCK_SIZE;

    pub const fn new() -> Self {
        Self { bitmap: [0u8; BLOCK_SIZE], last_free: 0 }
    }

    pub fn get_free_nodes_count(&self) -> u32 {
        let mut n = 0;
        for octet in self.bitmap.into_iter() {
            n += u8::count_zeros(octet);
        }
        n
    }

    pub fn take_free_block(&mut self) -> Option<Index> {
        for (index, value) in self.bitmap.iter_mut().skip(self.last_free).enumerate() {
            let slots = u8::trailing_ones(*value);
            if slots < 8 {
                *value |= 0b1 << slots;
                self.last_free += index;
                return Some((8 * (index + self.last_free) as u32) + slots);
            }
        }
        self.last_free = BLOCK_SIZE;
        None
    }

    pub const fn restore_free_block(&mut self, index: Index) {
        let shift = index % 8;
        let idx = (index / 8) as usize;
        self.bitmap[idx] ^= 0b1 << shift;
        self.last_free = idx;
    }
}

impl Serializable for Free {
    fn serialize(&self, out: &mut [u8]) -> Result<usize, Error> {
        out[0..512].copy_from_slice(&self.bitmap);
        Ok(512)
    }
}

impl Deserializable<Free> for Free {
    fn deserialize(buf: &[u8]) -> Result<Self, Error> {
        let mut free = Free::new();
        free.bitmap.copy_from_slice(&buf[0..512]);
        Ok(free)
    }
}

#[cfg(test)]
mod test {
    use crate::alloc_block_buffer;

    use super::*;

    fn take_nth_blocks(sut: &mut Free, n: usize) -> Option<Index> {
        let mut last = None;
        for _ in 0..n {
            last = sut.take_free_block();
        }
        last
    }

    #[test]
    fn get_free_nodes_count() {
        let sut = Free::new();
        assert_eq!(4096, sut.get_free_nodes_count())
    }

    #[test]
    fn take_free_blocks() {
        let mut sut = Free::new();
        assert_eq!(Some(0), sut.take_free_block());
        assert_eq!(Some(1), sut.take_free_block());
        assert_eq!(Some(2), sut.take_free_block());
        assert_eq!(Some(4095), take_nth_blocks(&mut sut, 4093));
        assert_eq!(0, sut.get_free_nodes_count());
    }

    #[test]
    fn restore_free_block() {
        let mut sut = Free::new();
        assert_eq!(Some(4095), take_nth_blocks(&mut sut, 4096));
        assert_eq!(0, sut.get_free_nodes_count());

        sut.restore_free_block(512);
        assert_eq!(1, sut.get_free_nodes_count());
        assert_eq!(Some(512), sut.take_free_block());

        assert_eq!(0, sut.get_free_nodes_count());

        sut.restore_free_block(4000);
        assert_eq!(1, sut.get_free_nodes_count());
        assert_eq!(Some(4000), sut.take_free_block());
    }

    #[test]
    fn serde_symmetry() {
        let mut expected = Free::new();
        take_nth_blocks(&mut expected, 2048);

        let mut buf = alloc_block_buffer();
        expected.serialize(&mut buf).expect("should serialize");

        let actual = Free::deserialize(&buf).expect("should deserialize");
        assert_eq!(expected.bitmap, actual.bitmap);
    }
}
