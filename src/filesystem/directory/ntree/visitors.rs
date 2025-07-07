use crate::{
    BlockDevice, Error,
    filesystem::{Addr, DirectoryNode, directory::entry::EntryKind},
};

pub trait Visitor {
    fn visit(&mut self, node: &DirectoryNode, depth: usize) -> Result<(), Error>;

    fn walk_tree<D: BlockDevice>(
        &mut self,
        device: &mut D,
        addr: Addr,
        depth: usize,
    ) -> Result<(), Error> {
        let current_node = DirectoryNode::load(device, addr)?;
        for entry in current_node.iter_entries().filter(|entry| entry.is_dir()) {
            self.walk_tree(device, entry.addr(), depth + 1)?;
        }
        self.visit(&current_node, depth)?;
        Ok(())
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

    pub fn visit(&mut self, node: &DirectoryNode, _depth: usize) -> Result<(), Error> {
        self.count += node.iter_entries().filter(|entry| entry.kind() == self.kind).count();
        Ok(())
    }

    pub const fn result(self) -> usize {
        self.count
    }
}

impl Visitor for CounterVisitor {
    fn visit(&mut self, node: &DirectoryNode, depth: usize) -> Result<(), Error> {
        self.visit(node, depth)
    }
}
