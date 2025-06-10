use crate::filesystem::Addr;

#[derive(Debug)]
pub struct Range(Addr, Addr);

impl Range {
    pub const fn new(begin: Addr, end: Addr) -> Self {
        assert!(begin < end, "Invalid range: begin must be less than end");

        Self(begin, end)
    }

    pub const fn begin(&self) -> Addr {
        self.0
    }

    pub const fn end(&self) -> Addr {
        self.1
    }

    pub const fn len(&self) -> usize {
        (self.end() - self.begin()) as usize
    }

    pub const fn nth(&self, nth: Addr) -> Addr {
        debug_assert!(self.begin() + nth < self.end(), "Address out of range");

        self.begin() + nth
    }

    pub const fn next(&self, shift: usize) -> Self {
        Self(self.end(), self.end() + shift as u32)
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
    #[should_panic(expected = "Address out of range")]
    fn range_nth_out_of_bounds() {
        let range = Range::new(0, 10);
        range.nth(10);
    }
}
