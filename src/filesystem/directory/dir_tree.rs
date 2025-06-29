use std::println;

use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        FileName, Layout,
        directory::{DirEntry, FileEntry},
        path,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct DirTree {}

impl DirTree {
    pub fn insert<D>(&self, device: &mut D, file_path: &str) -> Result<FileEntry, Error>
    where
        D: BlockDevice,
    {
        insert_file(device, file_path, 0)
    }

    pub fn remove<D>(&self, device: &mut D, file_path: &str) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        remove_file(device, file_path, 0)
    }

    pub fn get<D>(&self, device: &mut D, file_path: &str) -> Result<FileEntry, Error>
    where
        D: BlockDevice,
    {
        find_file(device, file_path, 0)
    }

    pub fn count_files<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        count_files(device, 0)
    }

    pub fn print_tree<D>(&self, device: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        print_tree_inner(device, 0, 0, 0)
    }
}

fn remove_file<D: BlockDevice>(device: &mut D, file_path: &str, addr: Addr) -> Result<(), Error> {
    let mut current = DirEntry::load(device, addr)?;
    if path::dirname(file_path) == current.name {
        if let Some(file) =
            current.files.iter_mut().find(|f| f.is_valid() && f.name() == path::basename(file_path))
        {
            *file = FileEntry::empty();
            current.store(device, addr)?;
            return Ok(());
        }
        return Err(Error::FileNotFound);
    }

    let is_root = addr == 0;
    for next_addr in current.dirs.into_iter().filter(|addr| *addr != 0) {
        let next_path = if is_root { file_path } else { path::tail(file_path) };
        if remove_file(device, next_path, next_addr).is_ok() {
            return Ok(());
        }
    }
    Err(Error::FileNotFound)
}

fn find_file<D: BlockDevice>(
    device: &mut D,
    file_path: &str,
    addr: Addr,
) -> Result<FileEntry, Error> {
    let mut current = DirEntry::load(device, addr)?;
    if path::dirname(file_path) == current.name {
        if let Some(file) =
            current.files.iter_mut().find(|f| f.is_valid() && f.name() == path::basename(file_path))
        {
            return Ok(file.clone());
        }
        return Err(Error::FileNotFound);
    }

    let is_root = addr == 0;
    for next_addr in current.dirs.into_iter().filter(|addr| *addr != 0) {
        let next_path = if is_root { file_path } else { path::tail(file_path) };
        if let Ok(file) = find_file(device, next_path, next_addr) {
            return Ok(file.clone());
        }
    }
    Err(Error::FileNotFound)
}

fn insert_file<D: BlockDevice>(
    device: &mut D,
    file_path: &str,
    addr: Addr,
) -> Result<FileEntry, Error> {
    let mut current = DirEntry::load(device, addr)?;

    // No directory left, do file insertion on the current entry.
    if path::dirname(file_path).is_empty() {
        if current.files.iter().any(|e| e.name() == file_path) {
            return Err(Error::FileAlreadyExists);
        }

        let pos = current.files.iter_mut().position(|f| !f.is_valid()).ok_or(Error::StorageFull)?;
        let file_addr = addr * DirEntry::MAX_CHILD_FILES as Addr + pos as Addr;
        let file_name = FileName::new(file_path)?;
        let file_entry = FileEntry::new(file_name, file_addr);
        current.files[pos] = file_entry.clone();
        current.store(device, addr)?;
        return Ok(file_entry);
    }

    // Otherwise, check the children directories to see if we need to follow it.
    let first_component = path::first_component(file_path);
    for next_addr in current.dirs.into_iter().filter(|a| *a != 0) {
        let dir = DirEntry::load(device, next_addr)?;
        if dir.name == first_component {
            return insert_file(device, path::tail(file_path), next_addr);
        }
    }

    // If we reach here, it means we need to create a new directory entry for the first component.
    // First check if the current node can fit another child directory.
    let dir_pointer = current.dirs.iter_mut().find(|addr| **addr == 0).ok_or(Error::StorageFull)?;
    let next_addr = find_free_addr_for_direntry(device)?;
    let entry = DirEntry::new(FileName::new(first_component).unwrap());
    entry.store(device, next_addr)?;

    // Persist current entry, and continue insertion in the new directory.
    *dir_pointer = next_addr;
    current.store(device, addr)?;
    insert_file(device, path::tail(file_path), next_addr)
}

fn find_free_addr_for_direntry<D: BlockDevice>(device: &mut D) -> Result<Addr, Error> {
    for (addr, _) in Layout::TREE.iter().skip(1) {
        let entry = DirEntry::load(device, addr)?;
        if entry.name.is_empty() {
            return Ok(addr);
        }
    }
    Err(Error::StorageFull)
}

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

    println!("{}{}/", "  ".repeat(depth), current_node.name.as_str());
    for child_idx in current_node.dirs.iter().filter(|a| **a != 0) {
        print_tree_inner(device, *child_idx, depth + 1, max_depth)?
    }

    for entry in current_node.files.iter().filter(|e| e.is_valid()) {
        println!("{}{}", "  ".repeat(depth + 2), entry.name().as_str());
    }

    Ok(())
}

fn count_files<D>(device: &mut D, addr: Addr) -> Result<usize, Error>
where
    D: BlockDevice,
{
    let current_node = DirEntry::load(device, addr)?;
    let mut count = current_node.files.iter().filter(|e| e.is_valid()).count();
    for addr in current_node.dirs.iter().filter(|a| **a != 0) {
        count += count_files(device, *addr)?;
    }
    Ok(count)
}
