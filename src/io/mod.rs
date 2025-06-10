pub use reader::Reader;
pub use writer::Writer;

use crate::Error;

mod reader;
mod writer;

/// Trait for writing bytes
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
