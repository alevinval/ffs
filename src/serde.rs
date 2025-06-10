use std::io;

pub trait Serializable {
    fn serialize(&self, out: &mut [u8]) -> io::Result<usize>;
}

pub trait Deserializable<T>
where
    T: Sized,
{
    fn deserialize(buf: &[u8]) -> io::Result<T>;
}
