use crate::Index;

#[derive(Debug)]
pub struct Range(Index, Index);

impl Range {
    pub const fn new(begin: Index, end: Index) -> Self {
        assert!(begin < end, "Invalid range: begin must be less than end");

        Self(begin, end)
    }

    pub const fn begin(&self) -> Index {
        self.0
    }

    pub const fn end(&self) -> Index {
        self.1
    }

    pub const fn nth(&self, nth: Index) -> Index {
        debug_assert!(self.begin() + nth < self.end(), "Index out of range");

        self.0 + nth
    }

    pub const fn rshift(&self, shift: Index) -> Self {
        Self(self.begin() + shift, self.end() + shift)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn range_creation() {
        let range = Range::new(0, 10);
        assert_eq!(range.begin(), 0);
        assert_eq!(range.end(), 10);
    }

    #[test]
    #[should_panic(expected = "Invalid range: begin must be less than end")]
    fn range_creation_panics() {
        Range::new(10, 10);
    }

    #[test]
    fn range_nth() {
        let range = Range::new(0, 10);
        assert_eq!(range.nth(5), 5);
    }

    #[test]
    #[should_panic(expected = "Index out of range")]
    fn range_nth_out_of_bounds() {
        let range = Range::new(0, 10);
        range.nth(10);
    }
}
