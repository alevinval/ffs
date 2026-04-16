use crate::{
    Addr, Deserializable, Error, FixedLen, Name, Serializable,
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    name: Name,
    addr: Addr,
    kind: DirEntryKind,
}

impl DirEntry {
    pub const fn empty() -> Self {
        Self { name: Name::empty(), addr: 0, kind: DirEntryKind::Dir }
    }

    pub const fn new(name: Name, addr: Addr, kind: DirEntryKind) -> Self {
        Self { name, addr, kind }
    }

    pub const fn is_dir(&self) -> bool {
        matches!(self.kind, DirEntryKind::Dir)
    }

    pub const fn kind(&self) -> DirEntryKind {
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
}

impl FixedLen for DirEntry {
    const BYTES_LEN: usize = Name::BYTES_LEN + size_of::<Addr>() + DirEntryKind::BYTES_LEN;
}

impl Serializable for DirEntry {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.name.serialize(writer)?;
        n += writer.write_addr(self.addr)?;
        n += self.kind.serialize(writer)?;
        Ok(n)
    }
}

impl Deserializable<Self> for DirEntry {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name = Name::deserialize(reader)?;
        let addr = reader.read_addr()?;
        let kind = DirEntryKind::deserialize(reader)?;
        Ok(Self { name, addr, kind })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirEntryKind {
    File,
    Dir,
}

impl FixedLen for DirEntryKind {
    const BYTES_LEN: usize = 1;
}

impl Serializable for DirEntryKind {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let kind_byte = match self {
            Self::File => 0,
            Self::Dir => 1,
        };
        writer.write_u8(kind_byte)?;
        Ok(1)
    }
}

impl Deserializable<Self> for DirEntryKind {
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
mod tests {

    use crate::test_serde_symmetry;

    use super::*;

    test_serde_symmetry!(DirEntry, DirEntry::new("test_file".into(), 1, DirEntryKind::File));
}
