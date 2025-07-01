pub use tree_node::TreeNode;

#[cfg(feature = "std")]
use std::println;

mod tree_node;

use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Layout, directory::Entry, path},
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirTree {}

impl DirTree {
    pub fn insert_file<D>(&self, device: &mut D, file_path: &str) -> Result<Entry, Error>
    where
        D: BlockDevice,
    {
        insert_file(device, file_path, 0)
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

    pub fn count_files<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        count_files(device, 0)
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
    if path::dirname(file_path).is_empty() {
        let basename = path::basename(file_path);
        if let Some(entry) = current.find_mut(basename) {
            *entry = Entry::empty();
            current.store(device, addr)?;
            return Ok(());
        }
        return Err(Error::FileNotFound);
    }

    let next_path = path::tail(file_path);
    let first_component = path::first_component(file_path);
    if let Some(entry) = current.find(first_component) {
        return remove_file(device, next_path, entry.addr());
    }
    Err(Error::FileNotFound)
}

fn get_file<D: BlockDevice>(device: &mut D, file_path: &str, addr: Addr) -> Result<Entry, Error> {
    let current = TreeNode::load(device, addr)?;
    if path::dirname(file_path).is_empty() {
        if let Some(entry) = current.find(path::basename(file_path)) {
            return Ok(entry.clone());
        }
        return Err(Error::FileNotFound);
    }

    let next_path = path::tail(file_path);
    let first_component = path::first_component(file_path);
    if let Some(dir_ref) = current.find(first_component) {
        return get_file(device, next_path, dir_ref.addr());
    }
    Err(Error::FileNotFound)
}

fn insert_file<D: BlockDevice>(
    device: &mut D,
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
        return insert_file(device, next_path, entry.addr());
    }

    // If we reach here, it means we need to create a new directory entry for the first component.
    // First check if the current node can fit another child directory.
    current.find_unset().ok_or(Error::StorageFull)?;
    let next_addr = find_free_addr_for_direntry(device)?;
    current.insert_node(first_component, next_addr)?;

    let entry = if path::dirname(path::tail(file_path)).is_empty() {
        TreeNode::new_leaf()
    } else {
        TreeNode::new()
    };
    entry.store(device, next_addr)?;
    current.store(device, addr)?;

    insert_file(device, next_path, next_addr)
}

fn find_free_addr_for_direntry<D: BlockDevice>(device: &mut D) -> Result<Addr, Error> {
    for (addr, _) in Layout::TREE.iter().skip(1) {
        let entry = TreeNode::load(device, addr)?;
        if entry.is_empty() {
            return Ok(addr);
        }
    }
    Err(Error::StorageFull)
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
        for entry in current_node.iter_set() {
            println!("{}{}", "  ".repeat(depth + 2), entry.name().as_str());
        }
    } else {
        for entry in current_node.iter_set() {
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
        Ok(current_node.iter_set().count())
    } else {
        let mut count = 0;
        for dir_ref in current_node.iter_set() {
            count += count_files(device, dir_ref.addr())?;
        }
        Ok(count)
    }
}
