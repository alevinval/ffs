use core::fmt;

pub use reader::Reader;
pub use writer::Writer;

use crate::filesystem::Addr;

mod reader;
mod writer;

#[cfg(feature = "std")]
pub struct StdoutFmtWriter;

impl fmt::Write for StdoutFmtWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use std::io::{self, Write};
        io::stdout().write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}

pub enum Error {
    /// The provided buffer is too small to fit the expected data.
    BufferTooSmall { expected: usize, found: usize },

    /// An underlying I/O error occurred.
    IO { io: std::io::Error },
}

/// Trait `Write` writes data to a destination.
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;

    fn write_u8(&mut self, value: u8) -> Result<usize, Error> {
        self.write(&[value])
    }

    fn write_u16(&mut self, value: u16) -> Result<usize, Error> {
        self.write(&value.to_le_bytes())
    }

    fn write_addr(&mut self, addr: Addr) -> Result<usize, Error> {
        self.write(&addr.to_le_bytes())
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

    fn read_addr(&mut self) -> Result<Addr, Error> {
        let mut buf = [0u8; size_of::<Addr>()];
        self.read(&mut buf)?;
        Ok(Addr::from_le_bytes(buf))
    }
}
