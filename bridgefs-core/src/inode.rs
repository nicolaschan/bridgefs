use bincode::{Decode, Encode};

#[derive(Encode, Decode, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct INode(u64);

impl INode {
    pub fn new(inode: u64) -> Self {
        INode(inode)
    }

    pub fn get(&self) -> u64 {
        self.0
    }

    pub fn next_inode(&mut self) -> INode {
        INode(self.0 + 1)
    }
}

impl Default for INode {
    fn default() -> Self {
        INode(2)
    }
}

impl From<u64> for INode {
    fn from(inode: u64) -> Self {
        INode::new(inode)
    }
}

impl From<INode> for u64 {
    fn from(inode: INode) -> Self {
        inode.get()
    }
}
