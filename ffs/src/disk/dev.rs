use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::FileExt,
    println,
};

use crate::{BlockDevice, Error, filesystem::Addr, io};

pub struct Device {
    file: File,
}

impl Device {
    pub fn new(path: &str) -> Result<Self, Error> {
        let file = OpenOptions::new().read(true).write(true).open(path).map_err(|e| {
            println!("Failed to open device at {path}: {e}");
            io::Error::IO { io: e }
        })?;
        Ok(Self { file })
    }
}

impl BlockDevice for Device {
    fn read(&mut self, sector: Addr, buf: &mut [u8]) -> Result<(), Error> {
        Ok(self
            .file
            .read_at(buf, 512 * u64::from(sector))
            .map(|_| ())
            .map_err(|e| io::Error::IO { io: e })?)
    }

    fn write(&mut self, sector: Addr, buf: &[u8]) -> Result<(), Error> {
        Ok(self
            .file
            .write_at(buf, 512 * u64::from(sector))
            .map(|_| ())
            .map_err(|e| io::Error::IO { io: e })
            .and_then(|()| self.file.flush().map_err(|e| io::Error::IO { io: e }))?)
    }
}
