#[cfg(feature = "std")]
use std::println;

use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr,
        allocator::Allocator,
        directory::{Entry, ntree::TreeNode},
        path,
    },
};

#[derive(Debug)]
pub struct Tree {
    allocator: Allocator,
}

impl Tree {
    pub const fn new(allocator: Allocator) -> Self {
        Self { allocator }
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
        count_files(device, 0)
    }

    pub fn count_dirs<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        count_dirs(device, 0)
    }

    #[cfg(feature = "std")]
    pub fn print_tree<D>(&self, device: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        print_tree_inner(device, 0, 0, 0)
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

#[cfg(feature = "std")]
fn print_tree_inner<D: BlockDevice>(
    device: &mut D,
    addr: Addr,
    depth: usize,
    max_depth: usize,
) -> Result<(), Error> {
    if max_depth > 0 && depth >= max_depth {
        return Ok(());
    }

    let current_node = TreeNode::load(device, addr)?;

    if addr == 0 {
        println!("$/")
    }

    if current_node.is_leaf() {
        for entry in current_node.iter_entries() {
            println!("{}{}", "  ".repeat(depth + 2), entry.name().as_str());
        }
    } else {
        for entry in current_node.iter_entries() {
            println!("{}{}/", "  ".repeat(depth + 1), entry.name().as_str());
            print_tree_inner(device, entry.addr(), depth + 1, max_depth)?
        }
    }

    Ok(())
}

fn count_files<D>(device: &mut D, addr: Addr) -> Result<usize, Error>
where
    D: BlockDevice,
{
    let current_node = TreeNode::load(device, addr)?;
    if current_node.is_leaf() {
        Ok(current_node.iter_entries().count())
    } else {
        let mut count = 0;
        for dir_ref in current_node.iter_entries() {
            count += count_files(device, dir_ref.addr())?;
        }
        Ok(count)
    }
}

fn count_dirs<D>(device: &mut D, addr: Addr) -> Result<usize, Error>
where
    D: BlockDevice,
{
    let current_node = TreeNode::load(device, addr)?;
    if current_node.is_leaf() {
        return Ok(1);
    }
    let mut count = 1;
    for entry in current_node.iter_entries() {
        count += count_dirs(device, entry.addr())?;
    }
    Ok(count)
}

#[cfg(test)]
mod test {
    use crate::{disk::MemoryDisk, filesystem::layout::Layout};

    use super::*;

    #[test]
    fn multiple_tree_ops() {
        let mut device = MemoryDisk::new(512, 10000);
        let mut allocator = Allocator::new(Layout::new(0, 100));
        TreeNode::new().store(&mut device, 0).unwrap();
        assert_eq!(Ok(0), allocator.allocate(&mut device));

        let mut tree = Tree::new(allocator);
        println!("tree before insertion:");
        tree.print_tree(&mut device).unwrap();
        assert_eq!(1, tree.count_dirs(&mut device).unwrap());

        let _ = tree.insert_file(&mut device, "dir/second/third/file.txt").unwrap();
        println!("tree after insertion:");
        tree.print_tree(&mut device).unwrap();
        assert_eq!(4, tree.count_dirs(&mut device).unwrap());

        let _ = tree.get_file(&mut device, "dir/second/third/file.txt").unwrap();
        println!("tree after removal:");
        tree.print_tree(&mut device).unwrap();

        tree.remove_file(&mut device, "/dir/second/third/file.txt").unwrap();
        assert_eq!(
            Error::FileNotFound,
            tree.get_file(&mut device, "/dir/second/third/file.txt").unwrap_err()
        );

        assert_eq!(Ok(false), tree.prune(&mut device, 0));
        println!("tree after prune:");
        tree.print_tree(&mut device).unwrap();
        assert_eq!(1, tree.count_dirs(&mut device).unwrap());
    }
}
