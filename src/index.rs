use crate::{filename::Filename, hash_pointer::HashPointer, inode::INode};
use std::collections::HashMap;

use bincode::{Decode, Encode};

#[derive(Encode, Decode, Default)]
pub struct Index {
    pub next_inode: INode,
    pub children: HashMap<Filename, INode>,
    pub files: HashMap<INode, HashPointer>,
}

pub struct IndexMapping {
    pub inode: INode,
    pub name: Filename,
}

pub struct FileLookupResult {
    pub inode: INode,
    pub hash: HashPointer,
    pub name: Filename,
}

impl Index {
    pub fn add_child(&mut self, name: Filename, hash: HashPointer) -> INode {
        let inode = self.next_inode;
        self.children.insert(name, inode);
        self.files.insert(inode, hash);
        self.next_inode = self.next_inode.next();
        inode
    }

    pub fn remove_child(&mut self, name: &Filename) {
        let inode = self.children.remove(name);
        inode.map(|inode| self.files.remove(&inode));
    }

    pub fn update_child(&mut self, inode: INode, hash: HashPointer) {
        self.files.insert(inode, hash);
    }

    pub fn get_child_by_name(&self, name: &Filename) -> Option<FileLookupResult> {
        let inode = self.children.get(name)?;
        let hash = self.files.get(inode)?;
        let file_lookup_result = FileLookupResult {
            inode: *inode,
            hash: hash.clone(),
            name: name.clone(),
        };
        Some(file_lookup_result)
    }

    pub fn get_child_by_inode(&self, inode: &INode) -> Option<&HashPointer> {
        self.files.get(inode)
    }

    pub fn list_children(&self) -> Vec<IndexMapping> {
        self.children
            .iter()
            .map(|(name, inode)| IndexMapping {
                inode: *inode,
                name: name.clone(),
            })
            .collect()
    }
}
