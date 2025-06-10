use crate::Error;

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn read_to(&mut self, out: &mut [u8]) -> Result<(), Error> {
        let end = self.pos + out.len();
        if end > self.buf.len() {
            return Err(Error::BufferTooSmall { expected: end, found: self.buf.len() });
        }
        out.copy_from_slice(&self.buf[self.pos..end]);
        self.pos = end;
        Ok(())
    }

    pub fn read<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos + N;
        if end > self.buf.len() {
            return Err(Error::BufferTooSmall { expected: end, found: self.buf.len() });
        }
        let mut value = [0u8; N];
        value.copy_from_slice(&self.buf[self.pos..end]);
        self.pos = end;
        Ok(value)
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        self.read::<1>().map(|bytes| bytes[0])
    }

    pub fn read_u16(&mut self) -> Result<u16, Error> {
        let val = self.read::<2>()?;
        Ok(u16::from_le_bytes(val))
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        let val = self.read::<4>()?;
        Ok(u32::from_le_bytes(val))
    }
}
