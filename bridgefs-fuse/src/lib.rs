use std::{
    ffi::{OsStr, OsString},
    time::{Duration, SystemTime},
};

use bridgefs_core::{
    content_store::{ContentStore, ParsingContentStoreExt},
    file_record::{CommonAttrs, Record},
    hash_pointer::TypedHashPointerReference,
    index::Index,
    inode::INode,
};
use fuser::{
    Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyWrite, Request, TimeOrNow,
};
use libc::{ENOENT, ENOTDIR, ENOTEMPTY};

use crate::{
    baybridge_adapter::{BaybridgeAdapter, BaybridgeContentStore, BaybridgeHashPointerReference},
    bridgefs::BridgeFS,
    fuse_file_ext::{FuseErrorExt, FuseFileExt, FuseFileResponseExt},
    fuse_store_ext::FuseStoreExt,
};

pub mod baybridge_adapter;
pub mod bridgefs;
mod fuse_file_ext;
pub mod fuse_store_ext;

const TTL: Duration = Duration::ZERO;

impl<'a> BridgeFS<BaybridgeHashPointerReference<'a>, BaybridgeContentStore<'a>> {
    pub fn from_baybridge(adapter: &'a BaybridgeAdapter) -> Self {
        let mut store = adapter.content_store();
        let empty_root_dir = store.empty_root_dir();
        let index_hash = adapter.hash_pointer_reference(empty_root_dir);
        BridgeFS::new(index_hash, store)
    }
}

impl<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore> Filesystem
    for BridgeFS<IndexHashT, StoreT>
{
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let response = self.lookup_record_by_name(parent.into(), &name.into());
        match response {
            Ok(record) => {
                reply.entry(&TTL, &record.attrs(), 0);
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let response = self.lookup_record_by_inode(ino.into());
        match response {
            Ok(record) => {
                reply.attr(&TTL, &record.attrs());
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        match self.read_file_data_by_inode(ino.into(), offset as usize, size as usize) {
            Ok(response) => {
                reply.data(&response.datablock.data);
            }
            Err(e) => {
                reply.error(e.to_errno());
                return;
            }
        };
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let entries = match self.list_directory_by_inode(ino.into()) {
            Ok(entries) => entries,
            Err(e) => {
                reply.error(e.to_errno());
                return;
            }
        };

        for (i, entry) in entries
            .entries
            .into_iter()
            .enumerate()
            .skip(offset as usize)
        {
            let name: OsString = entry.name.into();
            if reply.add(ino, (i + 1) as i64, entry.record.inner.file_type(), name) {
                break;
            }
        }
        reply.ok();
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        match self.write_to_file(ino.into(), offset as usize, data) {
            Ok(written) => {
                reply.written(written as u32);
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }

    fn create(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let attributes = CommonAttrs::builder()
            .perm(get_permissions(mode, umask))
            .uid(req.uid())
            .gid(req.gid())
            .build();
        let response = self.create_file(parent.into(), name.into(), attributes);
        match response {
            Ok(file) => {
                reply.created(&TTL, &file.attrs(), 0, 0, 0);
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }

    fn setattr(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        ctime: Option<SystemTime>,
        _fh: Option<u64>,
        crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let mut index = self.get_index();
        let child_hash = index.get_child_by_inode(&ino.into());
        if child_hash.is_none() {
            reply.error(ENOENT);
            return;
        }
        let mut file_record: Record = self.store.get_parsed(child_hash.unwrap());
        if let Some(mode) = mode {
            file_record.mut_attrs().perm = mode as u16;
        }

        if let Some(atime) = atime {
            file_record.mut_attrs().atime = match atime {
                TimeOrNow::SpecificTime(t) => t,
                TimeOrNow::Now => SystemTime::now(),
            };
        }

        if let Some(mtime) = mtime {
            file_record.mut_attrs().mtime = match mtime {
                TimeOrNow::SpecificTime(t) => t,
                TimeOrNow::Now => SystemTime::now(),
            };
        }

        if let Some(ctime) = ctime {
            file_record.mut_attrs().ctime = ctime;
        }

        if let Some(crtime) = crtime {
            file_record.mut_attrs().crtime = crtime;
        }

        let new_file_record_hash = self.store.add_parsed(&file_record);
        index.update_child(ino.into(), new_file_record_hash);
        self.index_hash.set_typed(&self.store.add_parsed(&index));

        let attr = match file_record {
            Record::File(file_record) => file_record.attrs(ino.into()),
            Record::Directory(_directory_record) => todo!(),
        };
        reply.attr(&TTL, &attr);
    }

    fn mkdir(
        &mut self,
        req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        let attributes = CommonAttrs::builder()
            .perm(get_permissions(mode, umask))
            .uid(req.uid())
            .gid(req.gid())
            .build();
        let response = self.create_directory(parent.into(), name.into(), attributes);
        match response {
            Ok(directory) => {
                reply.entry(&TTL, &directory.attrs(), 0);
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut index = self.get_index();

        let parent_inode: INode = parent.into();
        let parent = self.get_record_by_inode(parent_inode);
        if parent.is_none() {
            reply.error(ENOENT);
            return;
        }
        let mut parent = match parent.unwrap() {
            Record::Directory(dir) => dir,
            _ => {
                reply.error(ENOTDIR);
                return;
            }
        };

        parent.remove(&name.into());
        let parent_hash = self.store.add_parsed(&Record::Directory(parent));
        index.update_child(parent_inode, parent_hash);
        self.index_hash.set_typed(&self.store.add_parsed(&index));
        reply.ok();
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let parent_inode = parent.into();
        let parent = self.get_record_by_inode(parent_inode);
        if parent.is_none() {
            reply.error(ENOENT);
            return;
        }
        let mut parent = match parent.unwrap() {
            Record::Directory(dir) => dir,
            _ => {
                reply.error(ENOTDIR);
                return;
            }
        };

        let target_inode = match parent.get(&name.into()) {
            Some(inode) => *inode,
            None => {
                reply.error(ENOENT);
                return;
            }
        };
        let target_record = self.get_record_by_inode(target_inode);
        if target_record.is_none() {
            reply.error(ENOENT);
            return;
        }
        let target_directory = match target_record.unwrap() {
            Record::Directory(dir) => dir,
            _ => {
                reply.error(ENOTDIR);
                return;
            }
        };
        if !target_directory.children.is_empty() {
            reply.error(ENOTEMPTY);
            return;
        }

        let mut index = self.get_index();
        parent.remove(&name.into());
        let parent_hash = self.store.add_parsed(&Record::Directory(parent));
        index.update_child(parent_inode, parent_hash);
        self.index_hash.set_typed(&self.store.add_parsed(&index));
        reply.ok();
    }
}

fn get_permissions(mode: u32, umask: u32) -> u16 {
    (mode & 0o7777 & !umask) as u16
}
