use std::vec::Vec;

use crate::{Addr, BlockDevice, Error};

pub struct MockDevice {
    pub reads: Vec<(Addr, Vec<u8>)>,
    pub writes: Vec<(Addr, Vec<u8>)>,
}

impl Default for MockDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDevice {
    pub const fn new() -> Self {
        Self { reads: Vec::new(), writes: Vec::new() }
    }

    pub fn assert_write(&self, n: usize, sector: Addr, data: &[u8]) {
        let write = &self.writes[n];
        assert_eq!(sector, write.0, "sector missmatch on write {}", n);
        assert_eq!(data, &write.1, "data missmatch on write {}", n);
    }
}

impl BlockDevice for MockDevice {
    fn read_block(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error> {
        self.reads.push((sector, buf.into()));
        for (s, write) in self.writes.iter() {
            if *s == sector {
                buf.copy_from_slice(write);
                break;
            }
        }
        Ok(())
    }

    fn write_block(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error> {
        self.writes.push((sector, buf.into()));
        Ok(())
    }
}
