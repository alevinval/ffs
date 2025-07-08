use core::fmt;

use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr,
        allocator::Allocator,
        directory::{
            Entry,
            entry::EntryKind,
            ntree::{
                TreeNode,
                visitors::{CounterVisitor, Visitor},
            },
        },
        layout::Layout,
        path,
    },
};

#[derive(Debug)]
pub struct Tree {
    allocator: Allocator,
}

impl Tree {
    pub const fn new(layout: Layout) -> Self {
        let allocator = Allocator::new(layout);
        Self { allocator }
    }

    pub fn format<D: BlockDevice>(&mut self, device: &mut D) -> Result<(), Error> {
        TreeNode::new().store(device, 0)?;
        self.allocator.allocate(device)?;
        Ok(())
    }

    pub fn insert_file<D>(&mut self, device: &mut D, file_path: &str) -> Result<Entry, Error>
    where
        D: BlockDevice,
    {
        insert_file(device, &mut self.allocator, file_path, 0)
    }

    pub fn remove_file<D>(&self, device: &mut D, file_path: &str) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        find_and_then(device, file_path, 0, |device, addr, parent, pos| {
            *parent.get_mut(pos) = Entry::empty();
            parent.store(device, addr)?;
            Ok(())
        })
    }

    pub fn get_file<D>(&self, device: &mut D, file_path: &str) -> Result<Entry, Error>
    where
        D: BlockDevice,
    {
        find_and_then(device, file_path, 0, |_device, _addr, parent, pos| {
            Ok(parent.get(pos).clone())
        })
    }

    pub fn prune<D>(&mut self, device: &mut D, addr: Addr) -> Result<bool, Error>
    where
        D: BlockDevice,
    {
        prune(device, &mut self.allocator, addr)
    }

    pub fn count_files<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        let mut counter = CounterVisitor::new(EntryKind::File);
        counter.walk_from_root(device, 0)?;
        Ok(counter.result())
    }

    pub fn count_dirs<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        let mut counter = CounterVisitor::new(EntryKind::Dir);
        counter.walk_from_root(device, 0)?;
        Ok(counter.result())
    }

    pub fn print_tree<D, W>(
        &self,
        device: &mut D,
        base_path: &str,
        depth: usize,
        out: &mut W,
    ) -> Result<(), Error>
    where
        D: BlockDevice,
        W: fmt::Write,
    {
        find_and_then(device, base_path, 0, |device, _addr, node, pos| {
            let entry = node.get(pos);
            if !entry.is_dir() {
                return Err(Error::DirectoryNotFound);
            }
            print_tree_in_order(device, entry.addr(), depth, 0, out)?;
            Ok(())
        })
    }

    #[cfg(feature = "std")]
    pub fn print_tree_stdout<D>(
        &self,
        device: &mut D,
        base_path: &str,
        depth: usize,
    ) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        use std::{println, string::String};

        let mut txt = String::new();
        self.print_tree(device, base_path, depth, &mut txt)?;
        println!("{txt}");
        Ok(())
    }
}

fn insert_file<D: BlockDevice>(
    device: &mut D,
    allocator: &mut Allocator,
    file_path: &str,
    addr: Addr,
) -> Result<Entry, Error> {
    let mut current = TreeNode::load(device, addr)?;
    if path::dirname(file_path).is_empty() {
        if current.find(file_path).is_some() {
            return Err(Error::FileAlreadyExists);
        }

        let entry = current.insert(file_path, addr, EntryKind::File);
        current.store(device, addr)?;
        return entry;
    }

    let next_path = path::tail(file_path);
    let first_component = path::first_component(file_path);
    if let Some(entry) = current.find(first_component) {
        return insert_file(device, allocator, next_path, entry.addr());
    }

    // If we reach here, it means we need to create a new directory entry for the first component.
    // First check if the current node can fit another child directory.
    current.find_unset().ok_or(Error::StorageFull)?;
    let next_addr = allocator.allocate(device)?;
    current.insert(first_component, next_addr, EntryKind::Dir)?;

    let entry = if path::dirname(path::tail(file_path)).is_empty() {
        TreeNode::new_leaf()
    } else {
        TreeNode::new()
    };
    entry.store(device, next_addr)?;
    current.store(device, addr)?;

    insert_file(device, allocator, next_path, next_addr)
}

fn prune<D: BlockDevice>(
    device: &mut D,
    allocator: &mut Allocator,
    addr: Addr,
) -> Result<bool, Error> {
    let mut current = TreeNode::load(device, addr)?;
    let mut dirty = false;
    for entry in current.iter_entries_mut().filter(|entry| entry.is_dir()) {
        if let Ok(pruned) = prune(device, allocator, entry.addr())
            && pruned
        {
            *entry = Entry::empty();
            dirty = true;
        }
    }
    if addr != 0 && current.iter_entries().count() == 0 {
        allocator.release(device, addr)?;
        return Ok(true);
    }
    if dirty {
        current.store(device, addr)?;
    }
    Ok(false)
}

fn print_tree_in_order<D: BlockDevice, W: fmt::Write>(
    device: &mut D,
    addr: Addr,
    max_depth: usize,
    depth: usize,
    out: &mut W,
) -> Result<(), Error> {
    if max_depth > 0 && depth >= max_depth {
        return Ok(());
    } else if depth == 0 {
        if addr == 0 {
            out.write_str("$/\n")?;
        } else {
            out.write_str("../\n")?;
        }
    }
    let node = TreeNode::load(device, addr)?;
    for entry in node.iter_entries().filter(|entry| entry.is_dir()) {
        out.write_fmt(format_args!("{}{}/\n", "  ".repeat(depth + 1), entry.name().as_str()))?;
        print_tree_in_order(device, entry.addr(), max_depth, depth + 1, out)?;
    }
    for entry in node.iter_entries().filter(|e| !e.is_dir()) {
        out.write_fmt(format_args!("{}{}\n", "  ".repeat(depth + 1), entry.name().as_str()))?;
    }
    Ok(())
}

fn find_and_then<F, R, D: BlockDevice>(
    device: &mut D,
    file_path: &str,
    addr: Addr,
    mut cb: F,
) -> Result<R, Error>
where
    F: FnMut(&mut D, Addr, &mut TreeNode, usize) -> Result<R, Error>,
{
    let mut node = TreeNode::load(device, addr)?;
    let first_component = path::first_component(file_path);
    if let Some(pos) = node.find_index(first_component) {
        let next_path = path::tail(file_path);
        if next_path == file_path {
            return cb(device, addr, &mut node, pos);
        }
        return find_and_then(device, next_path, node.get(pos).addr(), cb);
    }
    Err(Error::FileNotFound)
}

#[cfg(test)]
mod test {
    use std::{println, string::String};

    use crate::{
        disk::MemoryDisk,
        filesystem::{SerdeLen, layout::Layout},
    };

    use super::*;

    const TEST_LAYOUT: Layout = Layout::new(0, 10);

    fn get_sut() -> (MemoryDisk, Tree) {
        let mut device =
            MemoryDisk::new(512, TEST_LAYOUT.entries_count() as usize * TreeNode::SERDE_LEN);
        let mut tree = Tree::new(TEST_LAYOUT);
        tree.format(&mut device).expect("failed to format device");
        (device, tree)
    }

    fn find_entry_addr<D: BlockDevice>(
        device: &mut D,
        file_path: &str,
        addr: Addr,
    ) -> Result<Addr, Error> {
        find_and_then(device, file_path, addr, |_device, _addr, parent, pos| {
            Ok(parent.get(pos).addr())
        })
    }

    #[test]
    fn test_find_addr_for_path_root() {
        let (mut device, _) = get_sut();
        assert_eq!(Ok(0), find_entry_addr(&mut device, "", 0));
    }

    #[test]
    fn test_find_addr_for_path_missing() {
        let (mut device, _) = get_sut();
        assert_eq!(
            Err(Error::FileNotFound),
            find_entry_addr(&mut device, "missing/path/file.txt", 0)
        );
    }

    #[test]
    fn test_find_addr_for_path_found() {
        let (mut device, mut tree) = get_sut();
        tree.insert_file(&mut device, "some/path/file.txt").expect("cannot insert file");
        assert_eq!(Ok(0), find_entry_addr(&mut device, "", 0));
        assert_eq!(Ok(1), find_entry_addr(&mut device, "some", 0));
        assert_eq!(Ok(2), find_entry_addr(&mut device, "some/path", 0));
        assert_eq!(Ok(2), find_entry_addr(&mut device, "some/path/file.txt", 0));
    }

    #[test]
    fn multiple_tree_ops() {
        let (mut device, mut tree) = get_sut();
        println!("tree before insertion:");
        tree.print_tree_stdout(&mut device, "", 0).unwrap();
        assert_eq!(0, tree.count_dirs(&mut device).unwrap());

        let _ = tree.insert_file(&mut device, "dir/second/third/file.txt").unwrap();
        println!("tree after insertion:");
        tree.print_tree_stdout(&mut device, "", 0).unwrap();
        assert_eq!(3, tree.count_dirs(&mut device).unwrap());

        let _ = tree.get_file(&mut device, "dir/second/third/file.txt").unwrap();
        tree.remove_file(&mut device, "/dir/second/third/file.txt").unwrap();
        println!("tree after removal:");
        tree.print_tree_stdout(&mut device, "", 0).unwrap();

        assert_eq!(
            Error::FileNotFound,
            tree.get_file(&mut device, "/dir/second/third/file.txt").unwrap_err()
        );

        assert_eq!(Ok(false), tree.prune(&mut device, 0));
        println!("tree after prune:");
        tree.print_tree_stdout(&mut device, "", 0).unwrap();
        assert_eq!(0, tree.count_dirs(&mut device).unwrap());
    }

    #[test]
    fn test_print_tree() {
        let (mut device, mut tree) = get_sut();

        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        tree.insert_file(&mut device, "dir1/dir2/old.txt").expect("should insert file");
        tree.insert_file(&mut device, "dir1/dir2/dir3/file.txt").expect("shoud insert file");
        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "", 0, &mut actual));
        let expected = "$/
  dir1/
    dir2/
      dir3/
        file.txt
      old.txt
";
        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_print_tree_relative() {
        let (mut device, mut tree) = get_sut();

        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        let _ = tree.insert_file(&mut device, "dir1/dir2/dir3/file.txt");
        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "dir1/dir2", 0, &mut actual));
        let expected = "../
  dir3/
    file.txt
";
        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_print_tree_relative_and_max_depth() {
        let (mut device, mut tree) = get_sut();

        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        let _ = tree.insert_file(&mut device, "dir1/dir2/dir3/file.txt");
        let _ = tree.insert_file(&mut device, "dir1/dir3/file.txt");
        let _ = tree.insert_file(&mut device, "dir1/dir3/dir4/dir5/file.txt");
        let _ = tree.insert_file(&mut device, "dir1/file.txt");
        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "dir1", 2, &mut actual));
        let expected = "../
  dir2/
    dir3/
  dir3/
    dir4/
    file.txt
  file.txt
";
        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_print_file_fails() {
        let (mut device, mut tree) = get_sut();

        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        let _ = tree.insert_file(&mut device, "dir1/dir2/dir3/file.txt");
        let _ = tree.insert_file(&mut device, "dir1/dir3/file.txt");
        let _ = tree.insert_file(&mut device, "dir1/dir3/dir4/dir5/file.txt");
        let _ = tree.insert_file(&mut device, "dir1/file.txt");

        let mut out = String::new();
        let result = tree.print_tree(&mut device, "dir1/file.txt", 0, &mut out);
        assert_eq!(Err(Error::DirectoryNotFound), result)
    }
}
