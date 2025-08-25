use crate::{
    file_record::{DirectoryRecord, Record},
    filename::Filename,
    hash_pointer::TypedHashPointer,
    inode::INode,
};
use std::collections::HashMap;

use bincode::{Decode, Encode};

#[derive(Encode, Decode, Debug)]
pub struct Index {
    next_inode: INode,
    inode_mapping: HashMap<INode, TypedHashPointer<Record>>,
}

impl Index {
    pub fn new(root_inode: INode, root: TypedHashPointer<Record>) -> Self {
        let mut inode_mapping = HashMap::new();
        inode_mapping.insert(root_inode, root);
        Self {
            next_inode: INode::default(),
            inode_mapping,
        }
    }
}

#[derive(Encode, Decode, PartialEq, Eq, Clone)]
pub struct FileLookupResult {
    pub inode: INode,
    pub hash: TypedHashPointer<Record>,
    pub name: Filename,
}

impl Index {
    pub fn add_child(
        &mut self,
        parent: &mut DirectoryRecord,
        name: Filename,
        hash: TypedHashPointer<Record>,
    ) -> INode {
        let inode = self.next_inode;
        parent.insert(name, inode);
        self.inode_mapping.insert(inode, hash);
        self.next_inode = self.next_inode.next_inode();
        inode
    }

    pub fn update_child(&mut self, inode: INode, hash: TypedHashPointer<Record>) {
        self.inode_mapping.insert(inode, hash);
    }

    pub fn get_child_by_name(
        &self,
        parent: &DirectoryRecord,
        name: &Filename,
    ) -> Option<FileLookupResult> {
        let inode = parent.get(name)?;
        let hash = self.inode_mapping.get(inode)?;
        let file_lookup_result = FileLookupResult {
            inode: *inode,
            hash: hash.clone(),
            name: name.clone(),
        };
        Some(file_lookup_result)
    }

    pub fn get_child_by_inode(&self, inode: &INode) -> Option<&TypedHashPointer<Record>> {
        self.inode_mapping.get(inode)
    }
}
