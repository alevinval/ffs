use crate::{
    BlockDevice, Error,
    filesystem::{Addr, TreeNode, directory::entry::EntryKind},
};

pub trait Visitor {
    fn visit(&mut self, node: &TreeNode, depth: usize) -> Result<(), Error>;

    fn walk_tree<D: BlockDevice>(
        &mut self,
        device: &mut D,
        addr: Addr,
        depth: usize,
    ) -> Result<(), Error> {
        let current_node = TreeNode::load(device, addr)?;
        for entry in current_node.iter_entries().filter(|entry| entry.is_dir()) {
            self.walk_tree(device, entry.addr(), depth + 1)?;
        }
        self.visit(&current_node, depth)?;
        Ok(())
    }

    fn walk_from_root<D: BlockDevice>(
        &mut self,
        device: &mut D,
        depth: usize,
    ) -> Result<(), Error> {
        self.walk_tree(device, 0, depth)
    }
}

pub struct CounterVisitor {
    kind: EntryKind,
    count: usize,
}

impl CounterVisitor {
    pub const fn new(kind: EntryKind) -> Self {
        Self { kind, count: 0 }
    }

    pub fn visit(&mut self, node: &TreeNode, _depth: usize) -> Result<(), Error> {
        self.count += node.iter_entries().filter(|entry| entry.kind() == self.kind).count();
        Ok(())
    }

    pub const fn result(self) -> usize {
        self.count
    }
}

impl Visitor for CounterVisitor {
    fn visit(&mut self, node: &TreeNode, depth: usize) -> Result<(), Error> {
        self.visit(node, depth)
    }
}
