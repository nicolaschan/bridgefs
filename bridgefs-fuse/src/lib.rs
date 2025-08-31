use std::{
    ffi::{OsStr, OsString},
    time::{Duration, SystemTime},
};

use bridgefs_core::{
    bridgefs::BridgeFS, content_store::ContentStore, file_record::CommonAttrs,
    hash_pointer::TypedHashPointerReference, index::INodeIndex,
};
use fuser::{
    Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyWrite, Request, TimeOrNow,
};

use crate::{
    baybridge_adapter::{BaybridgeAdapter, BaybridgeContentStore, BaybridgeHashPointerReference},
    fuse_file_ext::{FuseErrorExt, FuseFileExt, FuseFileResponseExt},
    fuse_store_ext::FuseStoreExt,
};

pub mod baybridge_adapter;
mod fuse_file_ext;
pub mod fuse_store_ext;

const TTL: Duration = Duration::ZERO;

pub struct BridgeFSFuse<IndexHashT: TypedHashPointerReference<INodeIndex>, StoreT: ContentStore>(
    BridgeFS<IndexHashT, StoreT>,
);

impl<'a> BridgeFSFuse<BaybridgeHashPointerReference<'a>, BaybridgeContentStore<'a>> {
    pub fn from_baybridge(adapter: &'a BaybridgeAdapter) -> Self {
        let mut store = adapter.content_store();
        let empty_root_dir = store.empty_root_dir();
        let index_hash = adapter.hash_pointer_reference(empty_root_dir);
        let bridgefs = BridgeFS::new(index_hash, store);
        BridgeFSFuse(bridgefs)
    }
}

impl<IndexHashT: TypedHashPointerReference<INodeIndex>, StoreT: ContentStore> Filesystem
    for BridgeFSFuse<IndexHashT, StoreT>
{
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let response = self.0.lookup_record_by_name(parent.into(), &name.into());
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
        let response = self.0.lookup_record_by_inode(ino.into());
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
        match self
            .0
            .read_file_data_by_inode(ino.into(), offset as usize, size as usize)
        {
            Ok(response) => {
                reply.data(&response.datablock.data);
            }
            Err(e) => {
                reply.error(e.to_errno());
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
        let entries = match self.0.list_directory_by_inode(ino.into()) {
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
        match self.0.write_to_file(ino.into(), offset as usize, data) {
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
        let response = self.0.create_file(parent.into(), name.into(), attributes);
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
        let record = match self.0.lookup_record_by_inode(ino.into()) {
            Ok(record) => record,
            Err(e) => {
                reply.error(e.to_errno());
                return;
            }
        };
        let mut attributes = record.inner.common_attrs().clone();

        if let Some(mode) = mode {
            attributes.perm = mode as u16;
        }
        if let Some(atime) = atime {
            attributes.atime = match atime {
                TimeOrNow::SpecificTime(t) => t,
                TimeOrNow::Now => SystemTime::now(),
            };
        }
        if let Some(mtime) = mtime {
            attributes.mtime = match mtime {
                TimeOrNow::SpecificTime(t) => t,
                TimeOrNow::Now => SystemTime::now(),
            };
        }
        if let Some(ctime) = ctime {
            attributes.ctime = ctime;
        }
        if let Some(crtime) = crtime {
            attributes.crtime = crtime;
        }

        match self
            .0
            .update_attributes_by_inode(ino.into(), attributes.clone())
        {
            Ok(record) => reply.attr(&TTL, &record.attrs()),
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
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
        let response = self
            .0
            .create_directory(parent.into(), name.into(), attributes);
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
        match self.0.remove_file_by_name(parent.into(), &name.into()) {
            Ok(_) => {
                reply.ok();
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        match self.0.remove_directory_by_name(parent.into(), &name.into()) {
            Ok(_) => {
                reply.ok();
            }
            Err(e) => {
                reply.error(e.to_errno());
            }
        }
    }
}

fn get_permissions(mode: u32, umask: u32) -> u16 {
    (mode & 0o7777 & !umask) as u16
}
