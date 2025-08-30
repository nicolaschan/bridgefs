use std::{collections::HashMap, time::SystemTime};

use bincode::{Decode, Encode};

use crate::{
    data_block::DataBlock, filename::Filename, hash_pointer::TypedHashPointer, inode::INode,
};

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode)]
pub enum Record {
    File(FileRecord),
    Directory(DirectoryRecord),
}

impl Record {
    pub fn mut_attrs(&mut self) -> &mut CommonAttrs {
        match self {
            Record::File(file_record) => &mut file_record.common_attrs,
            Record::Directory(directory_record) => &mut directory_record.common_attrs,
        }
    }
}

impl From<FileRecord> for Record {
    fn from(value: FileRecord) -> Self {
        Record::File(value)
    }
}

impl From<DirectoryRecord> for Record {
    fn from(value: DirectoryRecord) -> Self {
        Record::Directory(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Default, bon::Builder)]
pub struct DirectoryRecord {
    #[builder(default = HashMap::new())]
    pub children: HashMap<Filename, INode>,
    pub common_attrs: CommonAttrs,
    pub parent: INode,
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct IndexMapping {
    pub inode: INode,
    pub name: Filename,
}

impl DirectoryRecord {
    pub fn insert(&mut self, filename: Filename, inode: INode) {
        self.children.insert(filename, inode);
    }

    pub fn remove(&mut self, filename: &Filename) -> Option<INode> {
        self.children.remove(filename)
    }

    pub fn get(&self, filename: &Filename) -> Option<&INode> {
        self.children.get(filename)
    }

    pub fn size(&self) -> usize {
        self.children.len()
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

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, bon::Builder)]
pub struct FileRecord {
    pub content_hash: TypedHashPointer<DataBlock>,
    pub size: u64,
    pub common_attrs: CommonAttrs,
}

/// Attributes that are shared between files and directories
#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, bon::Builder)]
pub struct CommonAttrs {
    #[builder(default = 0o755)]
    pub perm: u16,
    #[builder(default = 501)]
    pub uid: u32,
    #[builder(default = 20)]
    pub gid: u32,
    #[builder(default = SystemTime::now())]
    pub atime: SystemTime,
    #[builder(default = SystemTime::now())]
    pub mtime: SystemTime,
    #[builder(default = SystemTime::now())]
    pub ctime: SystemTime,
    #[builder(default = SystemTime::now())]
    pub crtime: SystemTime,
}

impl Default for CommonAttrs {
    fn default() -> CommonAttrs {
        CommonAttrs::builder().build()
    }
}
