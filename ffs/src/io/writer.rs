use crate::io::{Error, Write};

pub struct Writer<'a> {
    inner: &'a mut [u8],
    pos: usize,
}

impl<'a> Writer<'a> {
    pub const fn new(inner: &'a mut [u8]) -> Self {
        Writer { inner, pos: 0 }
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        if self.pos + buf.len() > self.inner.len() {
            return Err(Error::BufferTooSmall {
                expected: self.pos + buf.len(),
                found: self.inner.len(),
            });
        }

        self.inner[self.pos..self.pos + buf.len()].copy_from_slice(buf);
        self.pos += buf.len();
        Ok(buf.len())
    }
}

impl Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.write(buf)
    }
}
