use crate::{
    Error,
    filesystem::{
        Addr, Deserializable, Name, SerdeLen, Serializable,
        handle::{FileHandle, NodeHandle},
    },
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    name: Name,
    addr: Addr,
}

impl Entry {
    pub const fn empty() -> Self {
        Self { name: Name::empty(), addr: 0 }
    }

    pub const fn new(name: Name, addr: Addr) -> Self {
        Self { name, addr }
    }

    pub const fn name(&self) -> &Name {
        &self.name
    }

    pub const fn addr(&self) -> Addr {
        self.addr
    }

    pub const fn is_set(&self) -> bool {
        self.addr != 0
    }

    pub const fn get_handles(&self) -> (FileHandle, NodeHandle) {
        (FileHandle::new(self.addr), NodeHandle::new(self.addr))
    }
}

impl Default for Entry {
    fn default() -> Self {
        Self::empty()
    }
}

impl SerdeLen for Entry {
    const SERDE_LEN: usize = Name::SERDE_LEN + size_of::<Addr>();
}

impl Serializable for Entry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.name.serialize(writer)?;
        n += writer.write_addr(self.addr)?;
        Ok(n)
    }
}

impl Deserializable<Self> for Entry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name = Name::deserialize(reader)?;
        let addr = reader.read_addr()?;
        Ok(Self { name, addr })
    }
}

#[cfg(test)]
mod test {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(Entry, Entry::new("test_file".into(), 1));
}
