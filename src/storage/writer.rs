use crate::{
    BLOCK_SIZE, BlockDevice, Data, Error, File, Index, Meta, Node, alloc_block_buffer,
    serde::Serializable, storage::Ranges,
};

pub enum Writer<'a> {
    Meta(&'a Meta),
    File(&'a File),
    Node(&'a File, &'a Node),
    Data(&'a Data<'a>),
}

impl<'a> Writer<'a> {
    pub fn write<D>(&self, out: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        self.log();
        let block_index = self.get_block_index();
        let mut buf = alloc_block_buffer();
        let offset = self.get_byte_offset();
        if offset.is_some() {
            out.read_block(block_index, &mut buf)?;
        }
        self.serialize(&mut buf[offset.unwrap_or(0) as usize..])?;
        out.write_block(block_index, &buf)
    }

    pub fn erase<D>(&self, out: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        let block_index = self.get_block_index();
        out.write_block(block_index, &alloc_block_buffer())
    }

    pub fn write_chunks<D>(node: &Node, data: &'a [u8], out: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        for (chunk_idx, data_chunk) in data.chunks(BLOCK_SIZE).enumerate() {
            let block_index = node.get_block_indexes()[chunk_idx];
            let data = Data::new(block_index, data_chunk);
            Writer::Data(&data).write(out).map_err(|_| Error::FailedIO)?;
        }
        Ok(())
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
    fn serialize(&self, out: &mut [u8]) -> Result<usize, Error> {
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
        Writer::Meta(&Meta::new(123)).write(&mut disk).expect("should succeed");

        assert_eq!(512, disk.position());
    }

    #[test]
    fn write_files() {
        let mut disk = get_disk();
        let file = File::from_str("hello", 123).unwrap();
        Writer::File(&file).write(&mut disk).expect("should succeed");

        assert_eq!(64000, disk.position());
    }

    #[test]
    fn write_nodes() {
        let mut disk = get_disk();
        let file = File::from_str("hello", 123).unwrap();
        let node = &Node::new(123, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        Writer::Node(&file, node).write(&mut disk).expect("should succeed");

        assert_eq!(530432, disk.position());
    }

    #[test]
    fn write_data() {
        let mut disk = get_disk();
        let data = Data::new(123, "hello world".as_bytes());
        Writer::Data(&data).write(&mut disk).expect("should succeed");

        assert_eq!(1113600, disk.position());
    }
}
