use crate::{FREE_BLOCKS_COUNT, Index, MAX_FILES, storage::range::Range};

pub mod reader;
pub mod writer;

mod range;

pub struct Ranges {}

impl Ranges {
    pub const META: Range = Range::new(0, 1);
    pub const FILE: Range = Self::META.next(MAX_FILES as Index);
    pub const NODE: Range = Self::FILE.next(MAX_FILES as Index);
    pub const FREE: Range = Self::NODE.next(FREE_BLOCKS_COUNT as Index);
    pub const DATA: Range = Self::FREE.next(MAX_FILES as Index);
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_continuous_range(a: Range, b: Range) {
        assert!(a.end() == b.begin(), "range {:?} does not end where {:?} begins", a, b);
    }

    #[test]
    fn ranges_layout() {
        assert_continuous_range(Ranges::META, Ranges::FILE);
        assert_continuous_range(Ranges::FILE, Ranges::NODE);
        assert_continuous_range(Ranges::NODE, Ranges::FREE);
        assert_continuous_range(Ranges::FREE, Ranges::DATA);
    }
}
