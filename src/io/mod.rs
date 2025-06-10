pub use reader::Reader;
pub use writer::Writer;

use crate::Error;

mod reader;
mod writer;

/// Trait `Write` writes data to a destination.
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;

    fn write_u8(&mut self, value: u8) -> Result<usize, Error> {
        self.write(&[value])
    }

    fn write_u16(&mut self, value: u16) -> Result<usize, Error> {
        self.write(&value.to_le_bytes())
    }

    fn write_u32(&mut self, value: u32) -> Result<usize, Error> {
        self.write(&value.to_le_bytes())
    }
}

/// Trait `Read` reads data from a source.
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;

    fn read_u8(&mut self) -> Result<u8, Error> {
        let mut buf = [0u8; 1];
        self.read(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> Result<u16, Error> {
        let mut buf = [0u8; 2];
        self.read(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32, Error> {
        let mut buf = [0u8; 4];
        self.read(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
}
