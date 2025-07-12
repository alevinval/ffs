use crate::{
    BlockDevice, Error,
    filesystem::{
        Addr,
        allocator::Allocator,
        paths, storage,
        tree::{
            entry::Kind,
            visitors::{CounterVisitor, Visitor},
        },
    },
};
pub use entry::Entry;
pub use tree_node::TreeNode;

mod entry;
pub mod printer;
mod tree_node;
mod visitors;

#[derive(Debug)]
pub struct Tree;

impl Tree {
    pub fn format<D: BlockDevice>(device: &mut D, allocator: &mut Allocator) -> Result<(), Error> {
        storage::store(device, 0, &TreeNode::new())?;
        allocator.allocate(device)?;
        Ok(())
    }

    pub fn insert_file<D>(
        device: &mut D,
        allocator: &mut Allocator,
        file_path: &str,
    ) -> Result<Entry, Error>
    where
        D: BlockDevice,
    {
        insert_file(device, allocator, file_path, 0)
    }

    pub fn remove_file<D>(device: &mut D, file_path: &str) -> Result<(), Error>
    where
        D: BlockDevice,
    {
        find_and_then(device, file_path, 0, |device, addr, parent, pos| {
            *parent.get_mut(pos) = Entry::empty();
            storage::store(device, addr, parent)?;
            Ok(())
        })
    }

    pub fn get_file<D>(device: &mut D, file_path: &str) -> Result<Entry, Error>
    where
        D: BlockDevice,
    {
        find_and_then(device, file_path, 0, |_device, _addr, parent, pos| {
            Ok(parent.get(pos).clone())
        })
    }

    pub fn prune<D>(device: &mut D, allocator: &mut Allocator, addr: Addr) -> Result<bool, Error>
    where
        D: BlockDevice,
    {
        prune(device, allocator, addr)
    }

    pub fn count_files<D>(device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        let mut counter = CounterVisitor::new(Kind::File);
        counter.walk_from_root(device, 0)?;
        Ok(counter.result())
    }

    pub fn count_dirs<D>(device: &mut D) -> Result<usize, Error>
    where
        D: BlockDevice,
    {
        let mut counter = CounterVisitor::new(Kind::Dir);
        counter.walk_from_root(device, 0)?;
        Ok(counter.result())
    }
}

fn insert_file<D: BlockDevice>(
    device: &mut D,
    allocator: &mut Allocator,
    file_path: &str,
    addr: Addr,
) -> Result<Entry, Error> {
    let mut current: TreeNode = storage::load(device, addr)?;
    if paths::dirname(file_path).is_empty() {
        if current.find(file_path).is_some() {
            return Err(Error::FileAlreadyExists);
        }

        let entry = current.insert(file_path, addr, Kind::File);
        storage::store(device, addr, &current)?;
        return entry;
    }

    let next_path = paths::tail(file_path);
    let first_component = paths::first_component(file_path);
    if let Some(entry) = current.find(first_component) {
        return insert_file(device, allocator, next_path, entry.addr());
    }

    // If we reach here, it means we need to create a new directory entry for the first component.
    // First check if the current node can fit another child directory.
    current.find_unset().ok_or(Error::StorageFull)?;
    let next_addr = allocator.allocate(device)?;
    current.insert(first_component, next_addr, Kind::Dir)?;

    let entry = if paths::dirname(paths::tail(file_path)).is_empty() {
        TreeNode::new_leaf()
    } else {
        TreeNode::new()
    };
    storage::store(device, next_addr, &entry)?;
    storage::store(device, addr, &current)?;
    insert_file(device, allocator, next_path, next_addr)
}

fn prune<D: BlockDevice>(
    device: &mut D,
    allocator: &mut Allocator,
    addr: Addr,
) -> Result<bool, Error> {
    let mut current: TreeNode = storage::load(device, addr)?;
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
        storage::store(device, addr, &current)?;
    }
    Ok(false)
}

pub fn find_and_then<F, R, D: BlockDevice>(
    device: &mut D,
    file_path: &str,
    addr: Addr,
    mut cb: F,
) -> Result<R, Error>
where
    F: FnMut(&mut D, Addr, &mut TreeNode, usize) -> Result<R, Error>,
{
    let mut node: TreeNode = storage::load(device, addr)?;
    let first_component = paths::first_component(file_path);
    if let Some(pos) = node.find_index(first_component) {
        let next_path = paths::tail(file_path);
        if next_path == file_path {
            return cb(device, addr, &mut node, pos);
        }
        return find_and_then(device, next_path, node.get(pos).addr(), cb);
    }
    Err(Error::FileNotFound)
}

#[cfg(test)]
mod tests {
    use std::println;

    use crate::{
        disk::MemoryDisk,
        filesystem::{SerdeLen, layouts::Layout, tree::printer},
    };

    use super::*;

    const TEST_LAYOUT: Layout = Layout::new(0, 10);

    fn prepare() -> (MemoryDisk, Allocator) {
        let mut device =
            MemoryDisk::new(512, TEST_LAYOUT.entries_count() as usize * TreeNode::SERDE_LEN);
        let mut allocator = Allocator::new(TEST_LAYOUT);
        Tree::format(&mut device, &mut allocator).expect("failed to format device");
        (device, allocator)
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
        let (mut device, _) = prepare();
        assert_eq!(Ok(0), find_entry_addr(&mut device, "", 0));
    }

    #[test]
    fn test_find_addr_for_path_missing() {
        let (mut device, _) = prepare();
        assert_eq!(
            Err(Error::FileNotFound),
            find_entry_addr(&mut device, "missing/path/file.txt", 0)
        );
    }

    #[test]
    fn test_find_addr_for_path_found() {
        let (mut device, mut allocator) = prepare();
        Tree::insert_file(&mut device, &mut allocator, "some/path/file.txt")
            .expect("cannot insert file");
        assert_eq!(Ok(0), find_entry_addr(&mut device, "", 0));
        assert_eq!(Ok(1), find_entry_addr(&mut device, "some", 0));
        assert_eq!(Ok(2), find_entry_addr(&mut device, "some/path", 0));
        assert_eq!(Ok(2), find_entry_addr(&mut device, "some/path/file.txt", 0));
    }

    #[test]
    fn multiple_tree_ops() {
        let (mut device, mut allocator) = prepare();
        println!("tree before insertion:");
        printer::print(&mut device, "", 0).unwrap();
        assert_eq!(0, Tree::count_dirs(&mut device).unwrap());

        let _ =
            Tree::insert_file(&mut device, &mut allocator, "dir/second/third/file.txt").unwrap();
        println!("tree after insertion:");
        printer::print(&mut device, "", 0).unwrap();
        assert_eq!(3, Tree::count_dirs(&mut device).unwrap());

        let _ = Tree::get_file(&mut device, "dir/second/third/file.txt").unwrap();
        Tree::remove_file(&mut device, "/dir/second/third/file.txt").unwrap();
        println!("tree after removal:");
        printer::print(&mut device, "", 0).unwrap();

        assert_eq!(
            Error::FileNotFound,
            Tree::get_file(&mut device, "/dir/second/third/file.txt").unwrap_err()
        );

        assert_eq!(Ok(false), Tree::prune(&mut device, &mut allocator, 0));
        println!("tree after prune:");
        printer::print(&mut device, "", 0).unwrap();
        assert_eq!(0, Tree::count_dirs(&mut device).unwrap());
    }
}
