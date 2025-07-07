use core::fmt;

use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr,
        allocator::Allocator,
        directory::{Entry, ntree::TreeNode},
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
        remove_file(device, file_path, 0)
    }

    pub fn get_file<D>(&self, device: &mut D, file_path: &str) -> Result<Entry, Error>
    where
        D: BlockDevice,
    {
        get_file(device, file_path, 0)
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
        let mut count = 0;
        let mut count_files = |node: &TreeNode, _depth: usize| {
            if node.is_leaf() {
                count += node.iter_entries().count();
            }
            Ok(())
        };
        visit_all_tree(device, 0, &mut count_files, 0)?;
        Ok(count)
    }

    pub fn count_dirs<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        let mut count = 0;
        let mut count_dirs = |node: &TreeNode, _detph: usize| {
            if !node.is_leaf() {
                count += node.iter_entries().count();
            }
            Ok(())
        };
        visit_all_tree(device, 0, &mut count_dirs, 0)?;
        Ok(count)
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
        let start_addr = find_addr_for_path(device, base_path, 0)?;
        let mut visitor = |node: &TreeNode, depth: usize| {
            if depth == 0 {
                if start_addr == 0 {
                    out.write_str("$/\n")?;
                } else {
                    out.write_str("../\n")?;
                }
            }
            if node.is_leaf() {
                for entry in node.iter_entries() {
                    out.write_fmt(format_args!(
                        "{}{}\n",
                        "  ".repeat(depth + 2),
                        entry.name().as_str()
                    ))?;
                }
            } else {
                for entry in node.iter_entries() {
                    out.write_fmt(format_args!(
                        "{}{}/\n",
                        "  ".repeat(depth + 1),
                        entry.name().as_str()
                    ))?;
                }
            }
            Ok(())
        };
        visit_all_tree(device, start_addr, &mut visitor, depth)?;
        Ok(())
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

fn remove_file<D: BlockDevice>(device: &mut D, file_path: &str, addr: Addr) -> Result<(), Error> {
    let mut current = TreeNode::load(device, addr)?;
    if current.is_leaf() {
        let basename = path::basename(file_path);
        if let Some(entry) = current.find_mut(basename) {
            *entry = Entry::empty();
            current.store(device, addr)?;
            return Ok(());
        }
        return Err(Error::FileNotFound);
    }

    let first_component = path::first_component(file_path);
    if let Some(entry) = current.find(first_component) {
        let next_path = path::tail(file_path);
        return remove_file(device, next_path, entry.addr());
    }
    Err(Error::FileNotFound)
}

fn get_file<D: BlockDevice>(device: &mut D, file_path: &str, addr: Addr) -> Result<Entry, Error> {
    let current = TreeNode::load(device, addr)?;
    if current.is_leaf() {
        if let Some(entry) = current.find(path::basename(file_path)) {
            return Ok(entry.clone());
        }
        return Err(Error::FileNotFound);
    }

    let first_component = path::first_component(file_path);
    if let Some(dir_ref) = current.find(first_component) {
        let next_path = path::tail(file_path);
        return get_file(device, next_path, dir_ref.addr());
    }
    Err(Error::FileNotFound)
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

        let entry = current.insert_file(file_path, addr);
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
    current.insert_node(first_component, next_addr)?;

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
    if current.is_leaf() {
        if current.iter_entries().count() == 0 {
            allocator.release(device, addr)?;
            return Ok(true);
        } else {
            return Ok(false);
        }
    }

    for entry in current.iter_entries_mut() {
        if let Ok(pruned) = prune(device, allocator, entry.addr())
            && pruned
        {
            *entry = Entry::empty();
        }
    }
    current.store(device, addr)?;

    if addr != 0 && current.iter_entries().count() == 0 {
        allocator.release(device, addr)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn find_addr_for_path<D: BlockDevice>(
    device: &mut D,
    mut file_path: &str,
    mut addr: Addr,
) -> Result<Addr, Error> {
    while !file_path.is_empty() {
        let current = TreeNode::load(device, addr)?;
        if current.is_leaf() {
            return Ok(addr);
        }

        let first_component = path::first_component(file_path);
        if let Some(entry) = current.find(first_component) {
            let next_path = path::tail(file_path);
            if next_path == file_path {
                return Ok(entry.addr());
            }
            file_path = next_path;
            addr = entry.addr();
        } else {
            return Err(Error::FileNotFound);
        }
    }
    Ok(addr)
}

fn visit_all_tree<V, D: BlockDevice>(
    device: &mut D,
    addr: Addr,
    visitor: &mut V,
    depth: usize,
) -> Result<(), Error>
where
    V: FnMut(&TreeNode, usize) -> Result<(), Error>,
{
    let current_node = TreeNode::load(device, addr)?;
    visitor(&current_node, depth)?;
    if !current_node.is_leaf() {
        for entry in current_node.iter_entries() {
            visit_all_tree(device, entry.addr(), visitor, depth + 1)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{println, string::String};

    use crate::{disk::MemoryDisk, filesystem::layout::Layout};

    use super::*;

    const TEST_LAYOUT: Layout = Layout::new(0, 100);

    fn get_sut() -> (MemoryDisk, Tree) {
        let mut device = MemoryDisk::new(512, 10000);
        let mut tree = Tree::new(TEST_LAYOUT);
        tree.format(&mut device).expect("failed to format device");
        (device, tree)
    }

    #[test]
    fn test_find_addr_for_path_root() {
        let (mut device, _) = get_sut();
        assert_eq!(Ok(0), find_addr_for_path(&mut device, "", 0));
    }

    #[test]
    fn test_find_addr_for_path_missing() {
        let (mut device, _) = get_sut();
        assert_eq!(
            Err(Error::FileNotFound),
            find_addr_for_path(&mut device, "missing/path/file.txt", 0)
        );
    }

    #[test]
    fn test_find_addr_for_path_found() {
        let (mut device, mut tree) = get_sut();
        tree.insert_file(&mut device, "some/path/file.txt").expect("cannot insert file");
        assert_eq!(Ok(0), find_addr_for_path(&mut device, "", 0));
        assert_eq!(Ok(1), find_addr_for_path(&mut device, "some", 0));
        assert_eq!(Ok(2), find_addr_for_path(&mut device, "some/path", 0));
        assert_eq!(Ok(2), find_addr_for_path(&mut device, "some/path/file.txt", 0));
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
        println!("tree after removal:");
        tree.print_tree_stdout(&mut device, "", 0).unwrap();

        tree.remove_file(&mut device, "/dir/second/third/file.txt").unwrap();
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

        let _ = tree.insert_file(&mut device, "dir1/dir2/dir3/file.txt");
        let mut actual = String::new();
        assert_eq!(Ok(()), tree.print_tree(&mut device, "", 0, &mut actual));
        let expected = "$/
  dir1/
    dir2/
      dir3/
          file.txt
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
}
