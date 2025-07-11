use crate::{
    Error,
    filesystem::{Addr, Deserializable, Name, SerdeLen, Serializable},
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    name: Name,
    addr: Addr,
    kind: Kind,
}

impl Entry {
    pub const fn empty() -> Self {
        Self { name: Name::empty(), addr: 0, kind: Kind::Dir }
    }

    pub const fn new(name: Name, addr: Addr, kind: Kind) -> Self {
        Self { name, addr, kind }
    }

    pub const fn is_dir(&self) -> bool {
        matches!(self.kind, Kind::Dir)
    }

    pub const fn kind(&self) -> Kind {
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

impl SerdeLen for Entry {
    const SERDE_LEN: usize = Name::SERDE_LEN + size_of::<Addr>() + Kind::SERDE_LEN;
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
        let kind = Kind::deserialize(reader)?;
        Ok(Self { name, addr, kind })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
}

impl SerdeLen for Kind {
    const SERDE_LEN: usize = 1;
}

impl Serializable for Kind {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let kind_byte = match self {
            Self::File => 0,
            Self::Dir => 1,
        };
        writer.write_u8(kind_byte)?;
        Ok(1)
    }
}

impl Deserializable<Self> for Kind {
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

    test_serde_symmetry!(Entry, Entry::new("test_file".into(), 1, Kind::File));
}
