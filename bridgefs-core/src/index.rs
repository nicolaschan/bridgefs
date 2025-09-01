use crate::{
    content_store::ContentStore,
    counting_store::{CountingStore, HasReferences},
    file_record::Record,
    hash_pointer::TypedHashPointer,
    inode::INode,
};
use std::collections::HashMap;

use bincode::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
pub struct INodeIndex {
    next_inode: INode,
    inode_mapping: HashMap<INode, TypedHashPointer<Record>>,
}

impl INodeIndex {
    pub fn new(root_inode: INode, root: TypedHashPointer<Record>) -> Self {
        let mut inode_mapping = HashMap::new();
        inode_mapping.insert(root_inode, root);
        Self {
            next_inode: INode::default(),
            inode_mapping,
        }
    }
}

impl INodeIndex {
    pub fn insert_new_inode(&mut self, hash: TypedHashPointer<Record>) -> INode {
        let inode = self.next_inode;
        self.inode_mapping.insert(inode, hash);
        self.next_inode = self.next_inode.next_inode();
        inode
    }

    pub fn update_inode(&mut self, inode: INode, hash: TypedHashPointer<Record>) {
        self.inode_mapping.insert(inode, hash);
    }

    pub fn lookup_inode(&self, inode: &INode) -> Option<&TypedHashPointer<Record>> {
        self.inode_mapping.get(inode)
    }
}

impl<StoreT: ContentStore> HasReferences<StoreT> for INodeIndex {
    fn delete_references(&self, _new_value: Option<&Self>, _store: &mut CountingStore<StoreT>) {
        // TODO: should we delete references?
    }
}
