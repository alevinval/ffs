pub use dir_entry::DirEntry;

#[cfg(feature = "std")]
use std::println;

mod dir_entry;

use crate::{
    BlockDevice, Error,
    filesystem::{Addr, Layout, Name, directory::FileRef, path},
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirTree {}

impl DirTree {
    pub fn insert_file<D>(&self, device: &mut D, file_path: &str) -> Result<FileRef, Error>
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

    pub fn get_file<D>(&self, device: &mut D, file_path: &str) -> Result<FileRef, Error>
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
    let mut current = DirEntry::load(device, addr)?;
    if path::dirname(file_path).is_empty() {
        if let Some(edge) = current.edges.iter_mut().find(|r| r.name() == path::basename(file_path))
        {
            edge.clear();
            current.store(device, addr)?;
            return Ok(());
        }
        return Err(Error::FileNotFound);
    }

    let first_component = path::first_component(file_path);
    if let Some(edge) = current.edges.iter().find(|r| r.name() == first_component) {
        return remove_file(device, path::tail(file_path), edge.addr());
    }
    Err(Error::FileNotFound)
}

fn get_file<D: BlockDevice>(device: &mut D, file_path: &str, addr: Addr) -> Result<FileRef, Error> {
    let current = DirEntry::load(device, addr)?;
    if path::dirname(file_path).is_empty() {
        if let Some(edge) = current.edges.iter().find(|r| r.name() == path::basename(file_path)) {
            return Ok(edge.clone());
        }
        return Err(Error::FileNotFound);
    }

    let first_component = path::first_component(file_path);
    if let Some(dir_ref) = current.edges.iter().find(|r| r.name() == first_component) {
        return get_file(device, path::tail(file_path), dir_ref.addr());
    }
    Err(Error::FileNotFound)
}

fn insert_file<D: BlockDevice>(
    device: &mut D,
    file_path: &str,
    addr: Addr,
) -> Result<FileRef, Error> {
    let mut current = DirEntry::load(device, addr)?;

    // No directory left, do file insertion on the current entry.
    if path::dirname(file_path).is_empty() {
        if current.edges.iter().any(|r| r.name() == file_path) {
            return Err(Error::FileAlreadyExists);
        }

        let file_ref = {
            let (pos, file_ref) = current
                .edges
                .iter_mut()
                .enumerate()
                .find(|(_, r)| !r.is_set())
                .ok_or(Error::StorageFull)?;
            *file_ref = FileRef::new(
                Name::new(file_path)?,
                addr * DirEntry::MAX_EDGES as Addr + pos as Addr,
            );
            file_ref.clone()
        };
        current.store(device, addr)?;
        return Ok(file_ref);
    }

    // Otherwise, check the children directories to see if we need to follow it.
    let next_path = path::tail(file_path);
    let first_component = path::first_component(file_path);
    if let Some(edge) = current.edges.iter().find(|r| r.name() == first_component) {
        return insert_file(device, next_path, edge.addr());
    }

    // If we reach here, it means we need to create a new directory entry for the first component.
    // First check if the current node can fit another child directory.
    let file_ref = current.edges.iter_mut().find(|r| !r.is_set()).ok_or(Error::StorageFull)?;
    let next_addr = find_free_addr_for_direntry(device)?;

    if path::dirname(path::tail(file_path)).is_empty() {
        DirEntry::new_leaf().store(device, next_addr)?;
    } else {
        DirEntry::new_node().store(device, next_addr)?;
    }

    // Persist current entry, and continue insertion in the new directory.
    let name = Name::new(first_component).unwrap();
    *file_ref = FileRef::new(name, next_addr);
    current.store(device, addr)?;
    insert_file(device, next_path, next_addr)
}

fn find_free_addr_for_direntry<D: BlockDevice>(device: &mut D) -> Result<Addr, Error> {
    for (addr, _) in Layout::TREE.iter().skip(1) {
        let entry = DirEntry::load(device, addr)?;
        if entry.is_empty {
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

    let current_node = DirEntry::load(device, addr)?;

    if addr == 0 {
        println!("$/")
    }

    if current_node.is_leaf {
        for edge in current_node.edges.iter().filter(|r| r.is_set()) {
            println!("{}{}", "  ".repeat(depth + 2), edge.name().as_str());
        }
    } else {
        for edge in current_node.edges.iter().filter(|r| r.is_set()) {
            println!("{}{}/", "  ".repeat(depth + 1), edge.name().as_str());
            print_tree_inner(device, edge.addr(), depth + 1, max_depth)?
        }
    }

    Ok(())
}

fn count_files<D>(device: &mut D, addr: Addr) -> Result<usize, Error>
where
    D: BlockDevice,
{
    let current_node = DirEntry::load(device, addr)?;
    if current_node.is_leaf {
        Ok(current_node.edges.iter().filter(|r| r.is_set()).count())
    } else {
        let mut count = 0;
        for dir_ref in current_node.edges.iter().filter(|r| r.is_set()) {
            count += count_files(device, dir_ref.addr())?;
        }
        Ok(count)
    }
}
