#[cfg(feature = "std")]
pub use mem::MemoryDisk;

pub use dev::Device;

#[cfg(feature = "std")]
mod mem;

mod dev;
