use crate::{
    BlockDevice, Error,
    filesystem::{Addr, TreeNode, storage, tree::entry::Kind},
};

pub trait Visitor {
    fn visit(&mut self, node: &TreeNode, depth: usize) -> Result<(), Error>;

    fn walk_tree<D: BlockDevice>(
        &mut self,
        device: &mut D,
        addr: Addr,
        depth: usize,
    ) -> Result<(), Error> {
        let node: TreeNode = storage::load(device, addr)?;
        for entry in node.iter_entries().filter(|entry| entry.is_dir()) {
            self.walk_tree(device, entry.addr(), depth + 1)?;
        }
        self.visit(&node, depth)?;
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
    kind: Kind,
    count: usize,
}

impl CounterVisitor {
    pub const fn new(kind: Kind) -> Self {
        Self { kind, count: 0 }
    }

    pub fn visit(&mut self, node: &TreeNode, _depth: usize) {
        self.count += node.iter_entries().filter(|entry| entry.kind() == self.kind).count();
    }

    pub const fn result(self) -> usize {
        self.count
    }
}

impl Visitor for CounterVisitor {
    fn visit(&mut self, node: &TreeNode, depth: usize) -> Result<(), Error> {
        self.visit(node, depth);
        Ok(())
    }
}
