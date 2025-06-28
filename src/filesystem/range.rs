use crate::filesystem::Addr;

#[derive(Debug)]
pub(crate) struct Range {
    pub begin: Addr,
    pub end: Addr,
    pub block_per_entry: Addr,
}

impl Range {
    pub const fn new(begin: Addr, capacity: Addr) -> Self {
        Self::new_with_size(begin, capacity, 1)
    }

    pub const fn new_with_size(begin: Addr, capacity: Addr, blocks_per_entry: Addr) -> Self {
        debug_assert!(blocks_per_entry > 0, "Entry size must be greater than zero");

        let end = begin + capacity * blocks_per_entry;
        Self { begin, end, block_per_entry: blocks_per_entry }
    }

    pub const fn sector_count(&self) -> Addr {
        self.end - self.begin
    }

    pub const fn entries_count(&self) -> Addr {
        self.sector_count() / self.block_per_entry
    }

    pub const fn nth(&self, logical: Addr) -> Addr {
        debug_assert!(self.begin + logical < self.end, "Address out of range");

        self.begin + (logical * self.block_per_entry)
    }

    pub const fn next_range(&self, capacity: usize, entry_size: usize) -> Self {
        Self::new_with_size(self.end, capacity as Addr, entry_size as Addr)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Addr, Addr)> {
        (0..self.sector_count()).map(|addr| (addr, self.nth(addr)))
    }

    pub fn circular_iter(&self, offset: Addr) -> impl Iterator<Item = (Addr, Addr)> {
        self.iter()
            .map(move |(addr, _)| (addr + offset) % self.sector_count())
            .map(|addr| (addr, self.nth(addr)))
    }

    #[cfg(test)]
    pub const fn iter_sectors(&self) -> core::ops::Range<Addr> {
        self.begin..self.end
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn range_creation() {
        let range = Range::new(0, 10);
        assert_eq!(range.begin, 0);
        assert_eq!(range.end, 10);
        assert_eq!(range.sector_count(), 10);
        assert_eq!(range.entries_count(), 10);
    }

    #[test]
    fn iter() {
        let range = Range::new(5, 10);
        let mut iter = range.iter();
        assert_eq!(Some((0, 5)), iter.next());
        assert_eq!(Some((1, 6)), iter.next());
        assert_eq!(Some((2, 7)), iter.next());
        assert_eq!(Some((3, 8)), iter.next());
    }

    #[test]
    fn circular_iter() {
        let range = Range::new(5, 10);
        let mut iter = range.circular_iter(8);
        assert_eq!(Some((8, 13)), iter.next());
        assert_eq!(Some((9, 14)), iter.next());
        assert_eq!(Some((0, 5)), iter.next());
        assert_eq!(Some((1, 6)), iter.next());
    }

    #[test]
    fn iter_sectors() {
        let range = Range::new(5, 10);
        let iter = range.iter_sectors();
        assert_eq!(5, iter.start);
        assert_eq!(15, iter.end)
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

    mod range_with_size {
        use super::*;

        #[test]
        fn range_with_size_creation() {
            let range = Range::new_with_size(1, 10, 2);
            assert_eq!(range.begin, 1);
            assert_eq!(range.end, 21);
            assert_eq!(range.sector_count(), 20);
            assert_eq!(range.entries_count(), 10);
        }

        #[test]
        fn range_with_size_nth() {
            let range = Range::new_with_size(1, 10, 2);
            assert_eq!(range.nth(0), 1);
            assert_eq!(range.nth(1), 3);
            assert_eq!(range.nth(2), 5);
            assert_eq!(range.nth(3), 7);
        }

        #[test]
        fn iter() {
            let range = Range::new_with_size(1, 13, 4);
            let mut iter = range.iter();
            assert_eq!(Some((0, 1)), iter.next());
            assert_eq!(Some((1, 5)), iter.next());
            assert_eq!(Some((2, 9)), iter.next());
            assert_eq!(Some((3, 13)), iter.next());
        }

        #[test]
        fn circular_iter() {
            let range = Range::new_with_size(1, 13, 4);
            let mut iter = range.circular_iter(2);
            assert_eq!(Some((2, 9)), iter.next());
            assert_eq!(Some((3, 13)), iter.next());
            assert_eq!(Some((4, 17)), iter.next());
            assert_eq!(Some((5, 21)), iter.next());
        }
    }
}
