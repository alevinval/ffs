use std::io::{self, Read, Seek, Write};

use crate::{BLOCK_SIZE, BlockDevice, Error, Index};

/// Simulates an actual volume, but in memory.
///
/// This is useful for testing purposes, where we want to avoid writing to the actual disk.

#[derive(Debug)]
pub struct MemoryDisk {
    data: Box<[u8]>,
    pos: usize,
}

impl MemoryDisk {
    pub fn new(size: usize) -> Self {
        let data = vec![0u8; size].into_boxed_slice();
        Self { data, pos: 0 }
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
}

impl BlockDevice for MemoryDisk {
    fn read_block(&mut self, sector: Index, buf: &mut [u8]) -> Result<(), Error> {
        self.seek(io::SeekFrom::Start(BLOCK_SIZE as u64 * sector as u64))
            .map_err(|_| Error::FailedIO)?;
        self.read(buf).map_err(|_| Error::FailedIO).map(|_| ())
    }

    fn write_block(&mut self, sector: Index, buf: &[u8]) -> Result<(), Error> {
        self.seek(io::SeekFrom::Start(BLOCK_SIZE as u64 * sector as u64))
            .map_err(|_| Error::FailedIO)?;
        self.write(buf).map_err(|_| Error::FailedIO).map(|_| ())
    }
}

impl io::Read for MemoryDisk {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = buf.len().min(self.capacity());
        buf[..len].copy_from_slice(&self.data[self.pos..(self.pos + len)]);
        self.pos += len;
        Ok(len)
    }
}

impl io::Write for MemoryDisk {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        println!("[memory-disk] write: cursor={} len={}", self.pos, buf.len());
        let len = buf.len().min(self.capacity());
        self.data[self.pos..(self.pos + len)].copy_from_slice(&buf[..len]);
        self.pos += len;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for MemoryDisk {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(offset) => {
                if offset as usize >= self.capacity() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Seek out of bounds"));
                }
                self.pos = offset as usize;
                Ok(offset)
            }
            _ => {
                Err(io::Error::new(io::ErrorKind::Unsupported, "Only SeekFrom::Start is supported"))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::{Read, Seek, Write};

    use super::MemoryDisk;

    #[test]
    fn capacity() {
        let sut = MemoryDisk::new(1024);
        assert_eq!(1024, sut.capacity(), "disk capacity should be 1024 bytes");
    }

    #[test]
    fn slice() {
        let sut = MemoryDisk::new(1024);

        assert_eq!([0, 0, 0, 0], sut.slice(0, 4), "disk should be initialized with zeros");
    }

    #[test]
    fn position() {
        let sut = MemoryDisk::new(1024);
        assert_eq!(0, sut.position());
    }

    #[test]
    fn write() {
        let mut sut = MemoryDisk::new(1024);

        let result = sut.write(&[1, 2, 3, 4]);
        assert!(result.is_ok(), "should succeed");
        assert_eq!(4, result.unwrap(), "should write 4 bytes to the disk");
        assert_eq!([1, 2, 3, 4], sut.slice(0, 4), "disk should contain the written data");
        assert_eq!(4, sut.position());
    }

    #[test]
    fn write_seek_read() -> Result<(), std::io::Error> {
        let mut sut = MemoryDisk::new(1024);

        sut.write_all(&[1, 2, 3, 4])?;
        sut.seek(std::io::SeekFrom::Start(0))?;

        let mut buf = [0u8; 4];
        let result = sut.read(&mut buf);
        assert!(result.is_ok(), "should succeed");
        assert_eq!([1, 2, 3, 4], buf);

        Ok(())
    }
}
