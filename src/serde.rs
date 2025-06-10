use crate::Error;

pub trait Serializable {
    fn serialize(&self, out: &mut [u8]) -> Result<usize, Error>;
}

pub trait Deserializable<T>
where
    T: Sized,
{
    fn deserialize(buf: &[u8]) -> Result<T, Error>;
}
