use crate::{MAX_FILES, storage::range::Range};

pub mod reader;
pub mod writer;

mod range;

pub struct Ranges {}

impl Ranges {
    pub const META: Range = Range::new(0, 1);
    pub const FILE: Range = Range::new(0, MAX_FILES).rshift(1);
    pub const NODE: Range = Self::FILE.rshift(MAX_FILES);
    pub const DATA: Range = Self::NODE.rshift(MAX_FILES);
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
        assert_continuous_range(Ranges::NODE, Ranges::DATA);
    }
}
