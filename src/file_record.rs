use std::time::SystemTime;

use bincode::{Decode, Encode};
use fuser::{FileAttr, FileType};

use crate::{hash_pointer::HashPointer, inode::INode};

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, bon::Builder)]
pub struct FileRecord {
    pub content_hash: HashPointer,
    #[builder(default = 0o755)]
    pub perm: u16,
    #[builder(default = 0)]
    pub size: u64,
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

impl FileRecord {
    pub fn attr(&self, inode: INode) -> FileAttr {
        FileAttr {
            ino: inode.into(),
            size: self.size,
            blocks: 0,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.crtime,
            kind: FileType::RegularFile,
            perm: self.perm,
            nlink: 2,
            uid: self.uid, // Change to appropriate user ID
            gid: self.gid, // Change to appropriate group ID
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }
}
