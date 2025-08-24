use std::{collections::HashMap, time::SystemTime};

use bincode::{Decode, Encode};
use fuser::{FileAttr, FileType};

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

    pub fn attrs(&self, inode: INode) -> FileAttr {
        match self {
            Record::File(file_record) => file_record.file_attr(inode),
            Record::Directory(directory_record) => directory_record.dir_attr(inode),
        }
    }

    pub fn file_type(&self) -> FileType {
        match self {
            Record::File(_) => FileType::RegularFile,
            Record::Directory(_) => FileType::Directory,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, Default, bon::Builder)]
pub struct DirectoryRecord {
    #[builder(default = HashMap::new())]
    pub children: HashMap<Filename, INode>,
    pub common_attrs: CommonAttrs,
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

    pub fn dir_attr(&self, inode: INode) -> FileAttr {
        FileAttr {
            ino: inode.into(),
            size: 0,
            blocks: 0,
            atime: self.common_attrs.atime,
            mtime: self.common_attrs.mtime,
            ctime: self.common_attrs.ctime,
            crtime: self.common_attrs.crtime,
            kind: FileType::Directory,
            perm: self.common_attrs.perm,
            nlink: 2,
            uid: self.common_attrs.uid, // Change to appropriate user ID
            gid: self.common_attrs.gid, // Change to appropriate group ID
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
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

impl FileRecord {
    pub fn file_attr(&self, inode: INode) -> FileAttr {
        FileAttr {
            ino: inode.into(),
            size: self.size,
            blocks: 0,
            atime: self.common_attrs.atime,
            mtime: self.common_attrs.mtime,
            ctime: self.common_attrs.ctime,
            crtime: self.common_attrs.crtime,
            kind: FileType::RegularFile,
            perm: self.common_attrs.perm,
            nlink: 2,
            uid: self.common_attrs.uid, // Change to appropriate user ID
            gid: self.common_attrs.gid, // Change to appropriate group ID
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }
}
