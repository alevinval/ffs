/// Block size expected by the filesystem.
pub const BLOCK_SIZE: usize = 512;

/// Maximum length of a file name in bytes.
pub const NAME_LEN: usize = 45;

/// The number of data blocks a single file node can reference.
/// This limits the maximum file size and is used for serialization, allocation, and layout.
pub const NODE_DATA_BLOCKS_LEN: usize = 10;

/// Entries that can fit in a directory tree node.
pub const TREE_NODE_ENTRY_LEN: usize = 30;

/// The maximum file size (in bytes) that a single node can represent.
pub const MAX_FILE_SIZE: usize = NODE_DATA_BLOCKS_LEN * BLOCK_SIZE;
