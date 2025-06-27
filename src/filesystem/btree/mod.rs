use std::println;

use crate::{
    Error,
    filesystem::{directory::Entry, file_name::FileName},
};

const ARENA_LEN: usize = 5;
const FILES_LEN: usize = 32;
const NODES_LEN: usize = 32;

#[derive(Debug)]
struct Arena {
    nodes: [Option<BTreeNode>; ARENA_LEN],
}

impl Arena {
    fn new() -> Self {
        let mut nodes = [const { None }; ARENA_LEN];
        nodes[0] = BTreeNode::root();
        Self { nodes }
    }

    fn print_tree(&self) {
        self.print_tree_inner(0, FileName::empty());
    }

    fn print_tree_inner(&self, current_node: usize, acc: FileName) {
        println!("print_tree_inner: current_node = {}, acc = {}", current_node, acc.as_str());

        let sep = FileName::new("/").unwrap();
        let current_node = self.nodes[current_node].as_ref().expect("Current node must exist");
        let acc = acc + current_node.name + sep;
        println!("  > Node: {}", acc.as_str());

        for child_idx in current_node.children.iter().flatten() {
            self.print_tree_inner(*child_idx, acc);
        }

        for entry in current_node.files.iter().flatten() {
            let fullpath = acc + *entry.file_name();
            println!("  > File: {}", fullpath.as_str());
        }
    }

    fn mkdir(&mut self, path: &str) -> Result<(), Error> {
        self.mkdir_inner(0, path);
        Ok(())
    }

    fn mkdir_inner(&mut self, current_node: usize, path: &str) -> Result<(), Error> {
        println!("mkdir_inner: current_node = {current_node}, path = {path}");
        let mut iter = path.splitn(2, "/");
        let first = iter.next();
        let next = iter.next();
        println!("first={first:?}, next={next:?}");

        if next.is_none() {
            let node = self.nodes[current_node].as_mut().unwrap();
            node.files
                .iter_mut()
                .find(|f| f.is_none())
                .ok_or(Error::StorageFull)?
                .replace(Entry::new(FileName::new(first.unwrap()).unwrap(), 0));
            return Ok(());
        }

        let part = first.unwrap();

        for child in self.nodes[current_node].as_ref().unwrap().children.iter() {
            if let Some(idx) = child {
                let child_node = self.nodes[*idx].as_ref().unwrap();
                if child_node.name.as_str() == part {
                    return self.mkdir_inner(*idx, iter.next().unwrap());
                }
            }
        }

        // Create a new node
        let new_node = BTreeNode {
            name: FileName::new(part).unwrap(),
            children: [None; 32],
            files: [const { None }; FILES_LEN],
        };
        let new_idx = self.nodes.iter().position(Option::is_none).ok_or(Error::StorageFull)?;
        self.nodes[new_idx] = Some(new_node);

        let current_node_pos = self.nodes[current_node]
            .as_ref()
            .expect("Current node must exist")
            .children
            .iter()
            .position(Option::is_none)
            .ok_or(Error::StorageFull)?;

        self.nodes[current_node].as_mut().unwrap().children[current_node_pos] = Some(new_idx);
        self.mkdir_inner(new_idx, next.unwrap())
    }
}

#[derive(Debug)]
struct BTreeNode {
    name: FileName,
    children: [Option<usize>; NODES_LEN],
    files: [Option<Entry>; FILES_LEN],
}

impl BTreeNode {
    fn root() -> Option<Self> {
        Some(Self {
            name: FileName::new("").unwrap(),
            children: [None; NODES_LEN],
            files: [const { None }; FILES_LEN],
        })
    }
}

#[cfg(test)]
mod test {

    use std::println;

    use crate::filesystem::btree::Arena;

    #[test]
    fn asd() {
        let mut arena = Arena::new();
        assert_eq!(Ok(()), arena.mkdir("test/dir/subdir/hello.txt"));
        println!("{arena:?}");
        arena.print_tree();
        assert!(false);
    }
}
