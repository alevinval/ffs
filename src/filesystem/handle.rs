use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr, Addressable, Deserializable, EraseFrom, LoadFrom, block::Block, file::File,
        node::Node,
    },
};

pub type FileHandle = Handle<File>;
pub type NodeHandle = Handle<Node>;

pub struct Handle<T>
where
    T: Deserializable<T> + Addressable,
{
    addr: Addr,

    _marker: core::marker::PhantomData<T>,
}

impl<T> Handle<T>
where
    T: Deserializable<T> + Addressable,
{
    pub const fn new(addr: Addr) -> Self {
        Self { addr, _marker: core::marker::PhantomData }
    }
}

impl<D, T> LoadFrom<D> for Handle<T>
where
    D: BlockDevice,
    T: Deserializable<T> + Addressable,
{
    type Item = T;

    fn load_from(&self, device: &mut D) -> Result<Self::Item, Error> {
        let sector = T::layout().nth(self.addr);
        let mut block = Block::new();
        device.read(sector, &mut block)?;
        T::deserialize(&mut block.reader())
    }
}

impl<D, T> EraseFrom<D> for Handle<T>
where
    D: BlockDevice,
    T: Deserializable<T> + Addressable,
{
    fn erase_from(&self, device: &mut D) -> Result<(), Error> {
        let sector = T::layout().nth(self.addr);
        let block = Block::new();
        device.write(sector, &block)
    }
}

#[cfg(test)]
mod tests {

    use crate::{filesystem::Store, test_utils::MockDevice};

    use super::*;

    mod file_handle {

        use crate::filesystem::Layout;

        use super::*;

        #[test]
        fn store_and_load_from() {
            let mut device = MockDevice::new();
            let expected = File::new("some-file.txt".into(), 123);
            expected.store(&mut device).expect("should store");
            let sut = FileHandle::new(123);
            let actual = sut.load_from(&mut device).expect("should load");
            assert_eq!(expected, actual);
        }

        #[test]
        fn erase_from() {
            let mut device = MockDevice::new();
            let sut = FileHandle::new(123);
            sut.erase_from(&mut device).expect("should erase");
            device.assert_write(0, Layout::FILE.nth(123), &Block::new());
        }
    }

    mod node_handle {

        use crate::filesystem::{Layout, node_writer::NodeWriter};

        use super::*;

        #[test]
        fn store_and_load_from() {
            let mut device = MockDevice::new();
            let expected = Node::new(5084, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
            NodeWriter::new(0, &expected).store(&mut device).expect("should store");
            let sut = NodeHandle::new(0);
            let actual = sut.load_from(&mut device).expect("should load");
            assert_eq!(expected, actual);
        }

        #[test]
        fn erase_from() {
            let mut device = MockDevice::new();
            let sut = NodeHandle::new(15);
            sut.erase_from(&mut device).expect("should erase");
            device.assert_write(0, Layout::NODE.nth(15), &Block::new());
        }
    }
}
