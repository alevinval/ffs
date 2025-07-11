use core::fmt;

use crate::{
    BlockDevice, Error,
    filesystem::{Addr, TreeNode, tree},
};

pub fn print_to<D, W>(
    device: &mut D,
    base_path: &str,
    depth: usize,
    out: &mut W,
) -> Result<(), Error>
where
    D: BlockDevice,
    W: fmt::Write,
{
    tree::find_and_then(device, base_path, 0, |device, _addr, node, pos| {
        let entry = node.get(pos);
        if !entry.is_dir() {
            return Err(Error::DirectoryNotFound);
        }
        print_in_order(device, entry.addr(), depth, 0, out)?;
        Ok(())
    })
}

#[cfg(feature = "std")]
pub fn print<D>(device: &mut D, base_path: &str, depth: usize) -> Result<(), Error>
where
    D: BlockDevice,
{
    use std::{println, string::String};

    let mut txt = String::new();
    print_to(device, base_path, depth, &mut txt)?;
    println!("{txt}");
    Ok(())
}

fn print_in_order<D: BlockDevice, W: fmt::Write>(
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
        print_in_order(device, entry.addr(), max_depth, depth + 1, out)?;
    }
    for entry in node.iter_entries().filter(|e| !e.is_dir()) {
        out.write_fmt(format_args!("{}{}\n", "  ".repeat(depth + 1), entry.name().as_str()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::string::String;

    use crate::{
        disk::MemoryDisk,
        filesystem::{SerdeLen, allocator::Allocator, layouts::Layout, tree::Tree},
    };

    use super::*;

    fn prepare() -> (MemoryDisk, Allocator) {
        let mut device =
            MemoryDisk::new(512, TEST_LAYOUT.entries_count() as usize * TreeNode::SERDE_LEN);
        let mut allocator = Allocator::new(TEST_LAYOUT);
        Tree::format(&mut device, &mut allocator).expect("failed to format device");
        (device, allocator)
    }

    const TEST_LAYOUT: Layout = Layout::new(0, 10);

    #[test]
    fn test_print_tree() {
        let (mut device, mut allocator) = prepare();

        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        Tree::insert_file(&mut device, &mut allocator, "dir1/dir2/old.txt")
            .expect("should insert file");
        Tree::insert_file(&mut device, &mut allocator, "dir1/dir2/dir3/file.txt")
            .expect("shoud insert file");
        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "", 0, &mut actual));
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
        let (mut device, mut allocator) = prepare();

        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir2/dir3/file.txt");
        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "dir1/dir2", 0, &mut actual));
        let expected = "../
  dir3/
    file.txt
";
        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_print_tree_relative_and_max_depth() {
        let (mut device, mut allocator) = prepare();

        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir2/dir3/file.txt");
        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir3/file.txt");
        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir3/dir4/dir5/file.txt");
        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/file.txt");
        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "dir1", 2, &mut actual));
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
        let (mut device, mut allocator) = prepare();

        let mut actual = String::new();
        assert_eq!(Ok(()), print_to(&mut device, "", 0, &mut actual));
        assert_eq!("$/\n", &actual);

        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir2/dir3/file.txt");
        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir3/file.txt");
        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/dir3/dir4/dir5/file.txt");
        let _ = Tree::insert_file(&mut device, &mut allocator, "dir1/file.txt");

        let mut out = String::new();
        let result = print_to(&mut device, "dir1/file.txt", 0, &mut out);
        assert_eq!(Err(Error::DirectoryNotFound), result);
    }
}
