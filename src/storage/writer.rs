use std::io::{self, Cursor, SeekFrom};

use crate::{
    BLOCK_SIZE, Data, File, Index, Meta, Node, alloc_block_buffer,
    serde::Serializable,
    storage::{Ranges, reader::Reader},
};

pub enum Writer<'a> {
    Meta(&'a Meta),
    File(&'a File),
    Node(&'a File, &'a Node),
    Data(&'a Data<'a>),
}

impl<'a> Writer<'a> {
    pub fn write<T>(&self, out: &mut T) -> io::Result<usize>
    where
        T: io::Read + io::Seek + io::Write,
    {
        self.log();
        let block_index = self.get_block_index();
        let mut cursor = Cursor::new(alloc_block_buffer());
        if let Some(offset) = self.get_byte_offset() {
            Reader::read_block(out, block_index, cursor.get_mut())?;
            cursor.set_position(offset as u64);
        }
        self.serialize(cursor.get_mut())?;
        Self::write_block(out, block_index, cursor.get_ref())
    }

    pub fn erase<T>(&self, out: &mut T) -> io::Result<usize>
    where
        T: io::Write + io::Seek + io::Read,
    {
        let block_index = self.get_block_index();
        Self::write_block(out, block_index, &alloc_block_buffer())
    }

    pub fn write_chunks<T>(node: &Node, data: &'a [u8], out: &mut T) -> io::Result<usize>
    where
        T: io::Read + io::Seek + io::Write,
    {
        let mut n = 0;
        for (chunk_idx, data_chunk) in data.chunks(BLOCK_SIZE).enumerate() {
            let block_index = node.get_block_indexes()[chunk_idx];
            let data = Data::new(block_index, data_chunk);
            n += Writer::Data(&data).write(out)?;
        }
        Ok(n)
    }

    fn write_block<T>(out: &mut T, block_number: u32, data: &[u8]) -> io::Result<usize>
    where
        T: io::Write + io::Seek,
    {
        assert!(data.len() <= BLOCK_SIZE, "write block: data exceeds block size");

        let pos = block_number as u64 * BLOCK_SIZE as u64;
        println!("[write-block] block={} pos={}", block_number, pos);

        out.seek(SeekFrom::Start(pos))?;
        out.write(data)
    }

    const fn get_block_index(&self) -> Index {
        match self {
            Writer::Meta(_) => Ranges::META.begin(),
            Writer::File(file) => Ranges::FILE.nth(file.get_node_index()),
            Writer::Node(file, _) => {
                let n = file.get_node_index() / Node::NODES_PER_BLOCK as Index;
                Ranges::NODE.nth(n)
            }
            Writer::Data(data) => Ranges::DATA.nth(data.get_block_index()),
        }
    }

    const fn get_byte_offset(&self) -> Option<Index> {
        match self {
            Writer::Node(file, _) => Some(
                file.get_node_index() % Node::NODES_PER_BLOCK as Index * Node::NODE_SIZE as Index,
            ),
            _ => None,
        }
    }

    fn log(&self) {
        match self {
            Writer::Meta(meta) => {
                println!(
                    "[write-metadata] block_size={} file_address={} node_address={} data_address={}",
                    meta.get_block_size(),
                    meta.get_file_address(),
                    meta.get_node_address(),
                    meta.get_data_address()
                );
            }
            Writer::File(file) => {
                println!("[write-file] name={} node={}", file.get_name(), file.get_node_index());
            }
            Writer::Node(file, node) => {
                println!(
                    "[write-node] index={} file_size={} block_indexes={:?}",
                    file.get_node_index(),
                    node.get_file_size(),
                    node.get_block_indexes(),
                );
            }
            Writer::Data(data) => {
                println!("[write-data] index={}", data.get_block_index());
            }
        };
    }
}

impl Serializable for Writer<'_> {
    fn serialize(&self, out: &mut [u8]) -> io::Result<usize> {
        match self {
            Writer::Meta(metadata) => metadata.serialize(out),
            Writer::File(file) => file.serialize(out),
            Writer::Node(_, node) => node.serialize(out),
            Writer::Data(data) => data.serialize(out),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{BLOCK_SIZE, disk::MemoryDisk};

    use super::*;

    fn get_disk() -> MemoryDisk {
        MemoryDisk::new(Ranges::DATA.end() as usize * BLOCK_SIZE)
    }

    #[test]
    fn write_meta() {
        let mut disk = get_disk();
        let n = Writer::Meta(&Meta::new(123)).write(&mut disk).expect("should succeed");

        assert_eq!(512, disk.position());
        assert_eq!(BLOCK_SIZE, n);
    }

    #[test]
    fn write_files() {
        let mut disk = get_disk();
        let file = File::new("hello", 123);
        let n = Writer::File(&file).write(&mut disk).expect("should succeed");

        assert_eq!(64000, disk.position());
        assert_eq!(BLOCK_SIZE, n)
    }

    #[test]
    fn write_nodes() {
        let mut disk = get_disk();
        let file = File::new("hello", 123);
        let node = &Node::new(123, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let n = Writer::Node(&file, node).write(&mut disk).expect("should succeed");

        assert_eq!(530432, disk.position());
        assert_eq!(BLOCK_SIZE, n)
    }

    #[test]
    fn write_data() {
        let mut disk = get_disk();
        let data = Data::new(123, "hello world".as_bytes());
        let n = Writer::Data(&data).write(&mut disk).expect("should succeed");

        assert_eq!(1112576, disk.position());
        assert_eq!(BLOCK_SIZE, n)
    }
}
