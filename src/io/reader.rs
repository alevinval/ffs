use crate::{Error, io::Read};

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
}

impl Read for Reader<'_> {
    fn read(&mut self, out: &mut [u8]) -> Result<usize, Error> {
        let end = self.pos + out.len();
        if end > self.buf.len() {
            return Err(Error::BufferTooSmall { expected: end, found: self.buf.len() });
        }
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.pos = end;
        Ok(out.len())
    }
}
