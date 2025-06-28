use std::{
    boxed::Box,
    fs::File,
    io::{Read, Write},
    vec,
};

use crate::BlockDevice;
use crate::Error;
use crate::filesystem::Addr;

/// Simulates an actual volume, but in memory.
///
/// This is useful for testing purposes, where we want to avoid writing to the actual disk.

#[derive(Debug)]
pub struct MemoryDisk {
    block_size: usize,
    data: Box<[u8]>,
    pos: usize,
}

impl MemoryDisk {
    pub fn fit(sectors: u32) -> Self {
        Self::new(512, sectors as usize * 512)
    }

    pub fn new(block_size: usize, capacity: usize) -> Self {
        let data = vec![0u8; capacity].into_boxed_slice();
        Self { block_size, data, pos: 0 }
    }

    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        &self.data[start..end]
    }

    pub const fn position(&self) -> usize {
        self.pos
    }

    const fn capacity(&self) -> usize {
        self.data.len()
    }

    const fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let len = buf.len().min(self.capacity());
        buf[..len].copy_from_slice(&self.data[self.pos..(self.pos + len)]);
        self.pos += len;
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> Result<(), Error> {
        let len = buf.len().min(self.capacity());
        self.data[self.pos..(self.pos + len)].copy_from_slice(&buf[..len]);
        self.pos += len;
        Ok(())
    }

    pub fn persist_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.data)
    }

    pub fn load_from_file(block_size: usize, path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut data = std::vec::Vec::new();
        file.read_to_end(&mut data)?;
        Ok(Self { block_size, data: data.into_boxed_slice(), pos: 0 })
    }
}

impl BlockDevice for MemoryDisk {
    fn read_block(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error> {
        self.seek(self.block_size * sector as usize);
        self.read(buf)
    }

    fn write_block(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error> {
        self.seek(self.block_size * sector as usize);
        self.write(buf).map(|_| ())
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn capacity() {
        let sut = MemoryDisk::new(512, 1024);
        assert_eq!(1024, sut.capacity(), "disk capacity should be 1024 bytes");
    }

    #[test]
    fn slice() {
        let sut = MemoryDisk::new(512, 1024);

        assert_eq!([0, 0, 0, 0], sut.slice(0, 4), "disk should be initialized with zeros");
    }

    #[test]
    fn position() {
        let sut = MemoryDisk::new(512, 1024);
        assert_eq!(0, sut.position());
    }

    #[test]
    fn write() {
        let mut sut = MemoryDisk::new(512, 1024);

        sut.write(&[1, 2, 3, 4]).expect("should write");
        assert_eq!([1, 2, 3, 4], sut.slice(0, 4), "disk should contain the written data");
        assert_eq!(4, sut.position());
    }

    #[test]
    fn write_seek_read() {
        let mut sut = MemoryDisk::new(512, 1024);

        sut.write(&[1, 2, 3, 4]).expect("asd");
        sut.seek(0);

        let mut buf = [0u8; 4];
        let result = sut.read(&mut buf);
        assert!(result.is_ok(), "should succeed");
        assert_eq!([1, 2, 3, 4], buf);
    }
}
