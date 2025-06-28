use std::println;

use crate::{
    Addr, BlockDevice, Error,
    filesystem::{
        FileName, Layout,
        directory::{DirEntry, FileEntry},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct Directory {}

impl Directory {
    pub fn add_file<D>(&self, device: &mut D, file_name: FileName) -> Result<FileEntry, Error>
    where
        D: BlockDevice,
    {
        // mkdir(device, &FileName::new("var/mnt/logs/index.html")?)?;
        // mkdir(device, &FileName::new("var/mnt/disk/hi.txt")?)?;
        // mkdir(device, &FileName::new("etc/initd/readme.md")?)?;
        // mkdir(device, &FileName::new("usr/home/void")?)?;
        let entry = mkdir(device, &file_name)?;
        // print_tree(device)?;
        let entry = entry.ok_or(Error::StorageFull)?;
        Ok(entry)
    }

    pub fn remove_file<D>(&self, device: &mut D, file_name: &FileName) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        delete(device, file_name, 0)
    }

    pub fn find_file<D>(&self, device: &mut D, file_name: &FileName) -> Result<FileEntry, Error>
    where
        D: BlockDevice,
    {
        find(device, file_name, 0)
    }

    pub fn file_exists<D>(&self, device: &mut D, name: &FileName) -> bool
    where
        D: BlockDevice,
    {
        self.find_file(device, name).is_ok()
    }

    pub fn count_files<D>(&self, device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        count_files(device)
    }

    pub fn print_tree<D>(&self, device: &mut D) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        print_tree(device)
    }
}

fn delete<D: BlockDevice>(device: &mut D, file_name: &FileName, addr: Addr) -> Result<(), Error> {
    println!("deleting file: {}", file_name.as_str());

    let mut dir = DirEntry::load(device, addr)?;

    if file_name.dirname() == dir.name.as_str() {
        let basename = file_name.basename();
        if let Some(file) =
            dir.files.iter_mut().find(|e| e.is_valid() && e.name().as_str() == basename)
        {
            *file = FileEntry::empty();
            dir.store(device, addr)?;
            return Ok(());
        }

        return Err(Error::FileNotFound);
    }

    for next_dir in dir.dirs.into_iter().filter(|addr| *addr != 0) {
        let next = if addr == 0 { file_name } else { &file_name.inside() };
        if delete(device, next, next_dir).is_ok() {
            return Ok(());
        }
    }
    Err(Error::FileNotFound)
}

fn find<D: BlockDevice>(
    device: &mut D,
    file_name: &FileName,
    addr: Addr,
) -> Result<FileEntry, Error> {
    let mut dir = DirEntry::load(device, addr)?;

    println!("finding file: {} -> {}", file_name.as_str(), dir.name.as_str());
    if file_name.dirname() == dir.name.as_str() {
        let basename = file_name.basename();
        if let Some(file) =
            dir.files.iter_mut().find(|e| e.is_valid() && e.name().as_str() == basename)
        {
            return Ok(file.clone());
        }

        return Err(Error::FileNotFound);
    }

    for next_dir in dir.dirs.into_iter().filter(|addr| *addr != 0) {
        let next = if addr == 0 { file_name } else { &file_name.inside() };
        if let Ok(file) = find(device, next, next_dir) {
            return Ok(file.clone());
        }
    }

    Err(Error::FileNotFound)
}

fn mkdir<D: BlockDevice>(device: &mut D, path: &FileName) -> Result<Option<FileEntry>, Error> {
    mkdir_inner(device, 0, path)
}

fn mkdir_inner<D: BlockDevice>(
    device: &mut D,
    current_pos: Addr,
    path: &FileName,
) -> Result<Option<FileEntry>, Error> {
    let mut current = DirEntry::load(device, current_pos)?;

    let dirname = path.dirname();
    if dirname.is_empty() {
        if current.files.iter().any(|e| e.name() == path) {
            return Err(Error::FileAlreadyExists);
        }

        let file = current.files.iter_mut().find(|f| !f.is_valid()).ok_or(Error::StorageFull)?;
        file.update(*path, 0);

        let file = file.clone();
        current.store(device, current_pos)?;
        return Ok(Some(file));
    }

    // Check current children directories to see if the entry already exists
    let first_component = first_component(dirname);
    for dir_addr in current.dirs.into_iter().filter(|a| *a != 0) {
        let dir = DirEntry::load(device, dir_addr)?;
        if dir.name.as_str() == first_component {
            return mkdir_inner(device, dir_addr, &path.inside());
        }
    }

    // Find free address to store new entry
    let mut allocated_addr: Option<Addr> = None;
    for (addr, _) in Layout::TABLE.iter() {
        let entry = DirEntry::load(device, addr)?;
        if entry.name.is_empty() && addr != 0 {
            allocated_addr = Some(addr);
            break;
        }
    }
    let addr = allocated_addr.ok_or(Error::StorageFull)?;

    let dir = DirEntry::new(FileName::new(first_component).unwrap());
    dir.store(device, addr)?;

    let children = current.dirs.iter_mut().find(|a| **a == 0).ok_or(Error::StorageFull)?;
    *children = addr;
    current.store(device, current_pos)?;

    mkdir_inner(device, addr, &path.inside())
}

pub fn print_tree<D: BlockDevice>(device: &mut D) -> Result<(), Error> {
    print_tree_inner(device, 0, FileName::empty(), 0, 0)
}

fn print_tree_inner<D: BlockDevice>(
    device: &mut D,
    current_node: Addr,
    acc: FileName,
    depth: usize,
    max_depth: usize,
) -> Result<(), Error> {
    if max_depth > 0 && depth >= max_depth {
        return Ok(());
    }

    let sep = FileName::new("/").unwrap();
    let current_node = DirEntry::load(device, current_node)?;
    let acc = acc + current_node.name + sep;

    println!("{}{}/", "  ".repeat(depth), current_node.name.as_str());
    for child_idx in current_node.dirs.iter().filter(|a| **a != 0) {
        print_tree_inner(device, *child_idx, acc, depth + 1, max_depth)?
    }

    for entry in current_node.files.iter().filter(|e| e.is_valid()) {
        println!("{}{}", "  ".repeat(depth + 2), entry.name().as_str());
    }

    Ok(())
}

fn first_component(path: &str) -> &str {
    path.trim_start_matches('/').split('/').next().unwrap_or("")
}

fn count_files<D>(device: &mut D) -> Result<usize, Error>
where
    D: BlockDevice,
{
    count_files_inner(device, 0)
}

fn count_files_inner<D>(device: &mut D, current_node: Addr) -> Result<usize, Error>
where
    D: BlockDevice,
{
    let mut count = 0;

    let current_node = DirEntry::load(device, current_node)?;
    for child_idx in current_node.dirs.iter().filter(|a| **a != 0) {
        count += count_files_inner(device, *child_idx)?;
    }
    count += current_node.files.iter().filter(|e| e.is_valid()).count();
    Ok(count)
}
