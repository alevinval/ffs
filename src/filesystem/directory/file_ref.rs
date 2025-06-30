use crate::{
    Error,
    filesystem::{
        Addr, Deserializable, Name, SerdeLen, Serializable,
        handle::{FileHandle, NodeHandle},
    },
    io::{Read, Write},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRef {
    name: Name,
    addr: Addr,
}

impl FileRef {
    pub const fn empty() -> Self {
        Self { name: Name::empty(), addr: 0 }
    }

    pub const fn new(name: Name, addr: Addr) -> Self {
        Self { name, addr }
    }

    pub const fn name(&self) -> &Name {
        &self.name
    }

    pub const fn is_valid(&self) -> bool {
        !self.name.is_empty()
    }

    pub const fn addr(&self) -> Addr {
        self.addr
    }

    pub const fn clear(&mut self) {
        self.name = Name::empty();
        self.addr = 0;
    }

    pub const fn get_handles(&self) -> (FileHandle, NodeHandle) {
        (FileHandle::new(self.addr), NodeHandle::new(self.addr))
    }
}

impl Default for FileRef {
    fn default() -> Self {
        Self::empty()
    }
}

impl SerdeLen for FileRef {
    const SERDE_LEN: usize = Name::SERDE_LEN + size_of::<Addr>();
}

impl Serializable for FileRef {
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<usize, Error> {
        let mut n = self.name.serialize(writer)?;
        n += writer.write_addr(self.addr)?;
        Ok(n)
    }
}

impl Deserializable<Self> for FileRef {
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name = Name::deserialize(reader)?;
        let addr = reader.read_addr()?;
        Ok(Self { name, addr })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::filesystem::Block;

    #[test]
    fn serde_symmetry() {
        let mut block = Block::new();

        let expected = FileRef::new("test_file".into(), 1);
        assert_eq!(Ok(FileRef::SERDE_LEN), expected.serialize(&mut block.writer()));
        let actual = FileRef::deserialize(&mut block.reader()).unwrap();

        assert_eq!(expected, actual);
    }
}
