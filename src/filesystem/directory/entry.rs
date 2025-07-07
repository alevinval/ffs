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
    kind: EntryKind,
}

impl Entry {
    pub const fn empty() -> Self {
        Self { name: Name::empty(), addr: 0, kind: EntryKind::Dir }
    }

    pub const fn new(name: Name, addr: Addr, kind: EntryKind) -> Self {
        Self { name, addr, kind }
    }

    pub const fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Dir)
    }

    pub const fn kind(&self) -> EntryKind {
        self.kind
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
    const SERDE_LEN: usize = Name::SERDE_LEN + size_of::<Addr>() + EntryKind::SERDE_LEN;
}

impl Serializable for Entry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.name.serialize(writer)?;
        n += writer.write_addr(self.addr)?;
        n += self.kind.serialize(writer)?;
        Ok(n)
    }
}

impl Deserializable<Self> for Entry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name = Name::deserialize(reader)?;
        let addr = reader.read_addr()?;
        let kind = EntryKind::deserialize(reader)?;
        Ok(Self { name, addr, kind })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
}

impl SerdeLen for EntryKind {
    const SERDE_LEN: usize = 1;
}

impl Serializable for EntryKind {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let kind_byte = match self {
            Self::File => 0,
            Self::Dir => 1,
        };
        writer.write_u8(kind_byte)?;
        Ok(1)
    }
}

impl Deserializable<Self> for EntryKind {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let byte = reader.read_u8()?;
        match byte {
            0 => Ok(Self::File),
            1 => Ok(Self::Dir),
            _ => Err(Error::UnsupportedDevice),
        }
    }
}

#[cfg(test)]
mod test {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(Entry, Entry::new("test_file".into(), 1, EntryKind::File));
}
