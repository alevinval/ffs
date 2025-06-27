use crate::filesystem::{directory::FileEntry, file_name::FileName};

const FILES_LEN: usize = 32;
const NODES_LEN: usize = 32;

pub struct DirEntry {
    name: FileName,
    edges: [Option<usize>; NODES_LEN],
    files: [FileEntry; FILES_LEN],
}
