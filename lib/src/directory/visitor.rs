use crate::{Addr, BlockDevice, Error, TreeNode, directory::direntry::DirEntryKind, storage};

pub trait Visitor<D>
where
    D: BlockDevice,
{
    fn visit(&mut self, node: &TreeNode) -> Result<(), Error>;

    fn walk_from_root(&mut self, device: &mut D, max_depth: usize) -> Result<(), Error> {
        self.walk_tree(device, 0, 0, max_depth)
    }

    fn walk_tree(
        &mut self,
        device: &mut D,
        addr: Addr,
        current_depth: usize,
        max_depth: usize,
    ) -> Result<(), Error> {
        if max_depth != 0 && current_depth == max_depth {
            return Ok(());
        }

        let node: TreeNode = storage::load(device, addr)?;
        for entry in node.iter_entries().filter(|entry| entry.is_dir()) {
            self.walk_tree(device, entry.addr(), current_depth + 1, max_depth)?;
        }
        self.visit(&node)?;
        Ok(())
    }
}

pub struct CounterVisitor {
    kind: DirEntryKind,
    count: usize,
}

impl CounterVisitor {
    pub const fn new(kind: DirEntryKind) -> Self {
        Self { kind, count: 0 }
    }

    pub fn visit(&mut self, node: &TreeNode) {
        self.count += node.iter_entries().filter(|entry| entry.kind() == self.kind).count();
    }

    pub const fn result(self) -> usize {
        self.count
    }
}

impl<D> Visitor<D> for CounterVisitor
where
    D: BlockDevice,
{
    fn visit(&mut self, node: &TreeNode) -> Result<(), Error> {
        self.visit(node);
        Ok(())
    }
}
