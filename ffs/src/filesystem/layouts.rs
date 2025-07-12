use crate::filesystem::{
    Addr, SerdeLen, allocator::Bitmap, block::Block, node::Node, tree::TreeNode,
};

const N_TREE: usize = 100;
const N_FILE: usize = N_TREE * TreeNode::LEN;
const N_DATA: usize = Node::BLOCKS_PER_NODE * N_FILE;
const N_FREE: usize = N_DATA / Bitmap::SLOTS;

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub(crate) begin: Addr,
    end: Addr,
    blocks_per_entry: Addr,
}

impl Layout {
    pub const META: Self = Self::new(0, 1);
    pub const TREE_BITMAP: Self = next(Self::META, 1, 1);
    pub const DATA_BITMAP: Self = next(Self::TREE_BITMAP, N_FREE, 1);
    pub const TREE: Self = next(Self::DATA_BITMAP, N_TREE, TreeNode::SERDE_BLOCK_COUNT);
    pub const FILE: Self = next(Self::TREE, N_FILE, 1);
    pub const NODE: Self = next(Self::FILE, N_FILE, 1);
    pub const DATA: Self = next(Self::NODE, N_DATA, 1);

    pub const fn new(begin: Addr, capacity: Addr) -> Self {
        Self::new_with_size(begin, capacity, 1)
    }

    pub const fn new_with_size(begin: Addr, capacity: Addr, blocks_per_entry: Addr) -> Self {
        debug_assert!(blocks_per_entry > 0, "Entry size must be greater than zero");

        let end = begin + capacity * blocks_per_entry;
        Self { begin, end, blocks_per_entry }
    }

    pub const fn sector_count(&self) -> Addr {
        self.end - self.begin
    }

    pub const fn entries_count(&self) -> Addr {
        self.sector_count() / self.blocks_per_entry
    }

    pub const fn nth(&self, logical: Addr) -> Addr {
        assert!(self.begin + logical * self.blocks_per_entry < self.end, "Address out of range");

        self.begin + (logical * self.blocks_per_entry)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Addr, Addr)> {
        (0..self.entries_count()).map(|addr| (addr, self.nth(addr)))
    }

    pub fn circular_iter(&self, offset: Addr) -> impl Iterator<Item = (Addr, Addr)> {
        self.iter()
            .map(move |(addr, _)| (addr + offset) % self.entries_count())
            .map(|addr| (addr, self.nth(addr)))
    }

    pub const fn iter_sectors(&self) -> core::ops::Range<Addr> {
        self.begin..self.end
    }

    pub const fn size_in_bytes(&self) -> usize {
        self.sector_count() as usize * Block::LEN
    }
}

const fn next(prev: Layout, capacity: usize, entry_size: usize) -> Layout {
    Layout::new_with_size(prev.end, capacity as Addr, entry_size as Addr)
}

#[cfg(feature = "std")]
pub fn print() {
    use std::println;
    println!("Disk layout:");
    println!("  Meta: {:?} ({} bytes)", Layout::META, Layout::META.size_in_bytes());
    println!(
        "  TreeBitmap: {:?} ({} bytes)",
        Layout::TREE_BITMAP,
        Layout::TREE_BITMAP.size_in_bytes()
    );
    println!(
        "  DataBitmap: {:?} ({} bytes)",
        Layout::DATA_BITMAP,
        Layout::DATA_BITMAP.size_in_bytes()
    );
    println!("  Tree: {:?} ({} bytes)", Layout::TREE, Layout::TREE.size_in_bytes());
    println!("  File: {:?} ({} bytes)", Layout::FILE, Layout::FILE.size_in_bytes());
    println!("  Node: {:?} ({} bytes)", Layout::NODE, Layout::NODE.size_in_bytes());
    println!("  Data: {:?} ({} bytes)", Layout::DATA, Layout::DATA.size_in_bytes());
    println!();
}

#[cfg(test)]
mod tests {

    use super::*;

    fn assert_continuous_layout_range(a: Layout, b: Layout) {
        assert!(a.end == b.begin, "range {a:?} does not end where {b:?} begins");
    }

    #[test]
    fn layout_ranges_are_continuous() {
        assert_continuous_layout_range(Layout::META, Layout::TREE_BITMAP);
        assert_continuous_layout_range(Layout::TREE_BITMAP, Layout::DATA_BITMAP);
        assert_continuous_layout_range(Layout::DATA_BITMAP, Layout::TREE);
        assert_continuous_layout_range(Layout::TREE, Layout::FILE);
        assert_continuous_layout_range(Layout::FILE, Layout::NODE);
        assert_continuous_layout_range(Layout::NODE, Layout::DATA);
    }

    #[test]
    fn new_with_size() {
        let sut = Layout::new_with_size(2, 12, 4);
        assert_eq!(sut.begin, 2);
        assert_eq!(sut.end, 50);
        assert_eq!(sut.sector_count(), 48);
        assert_eq!(sut.entries_count(), 12);
    }

    #[test]
    fn iter() {
        let sut = Layout::new(5, 10);
        let mut iter = sut.iter();
        assert_eq!(Some((0, 5)), iter.next());
        assert_eq!(Some((1, 6)), iter.next());
        assert_eq!(Some((2, 7)), iter.next());
        assert_eq!(Some((3, 8)), iter.next());
    }

    #[test]
    fn circular_iter() {
        let sut = Layout::new(5, 10);
        let mut iter = sut.circular_iter(8);
        assert_eq!(Some((8, 13)), iter.next());
        assert_eq!(Some((9, 14)), iter.next());
        assert_eq!(Some((0, 5)), iter.next());
        assert_eq!(Some((1, 6)), iter.next());
    }

    #[test]
    fn iter_sectors() {
        let sut = Layout::new(5, 10);
        let iter = sut.iter_sectors();
        assert_eq!(5, iter.start);
        assert_eq!(15, iter.end);
    }

    #[test]
    fn nth() {
        let sut = Layout::new(0, 10);
        assert_eq!(sut.nth(5), 5);
    }

    #[test]
    #[should_panic(expected = "Address out of range")]
    fn nth_out_of_bounds() {
        Layout::new(0, 10).nth(10);
    }

    #[test]
    fn test_size_in_bytes() {
        let sut = Layout::new(0, 10);
        assert_eq!(sut.size_in_bytes(), 10 * Block::LEN);

        let sut = Layout::new_with_size(0, 10, 2);
        assert_eq!(sut.size_in_bytes(), 10 * Block::LEN * 2);
    }

    mod with_size {
        use super::*;

        #[test]
        fn nth() {
            let sut = Layout::new_with_size(1, 10, 2);
            assert_eq!(sut.nth(0), 1);
            assert_eq!(sut.nth(1), 3);
            assert_eq!(sut.nth(2), 5);
            assert_eq!(sut.nth(3), 7);
        }

        #[test]
        fn iter() {
            let sut = Layout::new_with_size(1, 13, 4);
            let mut iter = sut.iter();
            assert_eq!(Some((0, 1)), iter.next());
            assert_eq!(Some((1, 5)), iter.next());
            assert_eq!(Some((2, 9)), iter.next());
            assert_eq!(Some((3, 13)), iter.next());
        }

        #[test]
        fn circular_iter() {
            let sut = Layout::new_with_size(1, 13, 4);
            let mut iter = sut.circular_iter(2);
            assert_eq!(Some((2, 9)), iter.next());
            assert_eq!(Some((3, 13)), iter.next());
            assert_eq!(Some((4, 17)), iter.next());
            assert_eq!(Some((5, 21)), iter.next());
        }
    }
}
