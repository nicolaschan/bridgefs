use bridgefs_core::{
    file_record::{CommonAttrs, DirectoryRecord, FileRecord, Record},
    inode::INode,
};
use fuser::{FileAttr, FileType};

pub trait FuseFileExt {
    fn attrs(&self, inode: INode) -> FileAttr;
    fn file_type(&self) -> FileType;
}

impl FuseFileExt for Record {
    fn attrs(&self, inode: INode) -> FileAttr {
        match self {
            Record::File(file_record) => file_record.attrs(inode),
            Record::Directory(directory_record) => directory_record.attrs(inode),
        }
    }

    fn file_type(&self) -> FileType {
        match self {
            Record::File(file_record) => file_record.file_type(),
            Record::Directory(directory_record) => directory_record.file_type(),
        }
    }
}

impl FuseFileExt for FileRecord {
    fn attrs(&self, inode: INode) -> FileAttr {
        to_file_attrs(&self.common_attrs, self.size, FileType::RegularFile, inode)
    }

    fn file_type(&self) -> FileType {
        FileType::RegularFile
    }
}

impl FuseFileExt for DirectoryRecord {
    fn attrs(&self, inode: INode) -> FileAttr {
        to_file_attrs(
            &self.common_attrs,
            self.size() as u64,
            FileType::Directory,
            inode,
        )
    }

    fn file_type(&self) -> FileType {
        FileType::Directory
    }
}

fn to_file_attrs(common_attrs: &CommonAttrs, size: u64, kind: FileType, inode: INode) -> FileAttr {
    FileAttr {
        ino: inode.into(),
        size,
        blocks: 0,
        atime: common_attrs.atime,
        mtime: common_attrs.mtime,
        ctime: common_attrs.ctime,
        crtime: common_attrs.crtime,
        kind,
        perm: common_attrs.perm,
        nlink: 2,
        uid: common_attrs.uid, // Change to appropriate user ID
        gid: common_attrs.gid, // Change to appropriate group ID
        rdev: 0,
        flags: 0,
        blksize: 512,
    }
}
