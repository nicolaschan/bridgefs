use baybridge::client::Actions;
use baybridge::configuration::Configuration;
use bridgefs_core::content_store::{ContentStore, ParsingContentStoreExt};
use bridgefs_core::data_block::DataBlock;
use bridgefs_core::file_record::{CommonAttrs, DirectoryRecord, FileRecord, Record};
use bridgefs_core::filename::Filename;
use bridgefs_core::hash_pointer::{HashPointerReference, TypedHashPointerReference};
use bridgefs_core::index::Index;
use bridgefs_core::inode::INode;
use bridgefs_core::manifest::Manifest;
use fuser::{
    FUSE_ROOT_ID, FileType, Filesystem, MountOption, ReplyAttr, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyWrite, Request, TimeOrNow,
};
use libc::{EISDIR, ENOENT, ENOTDIR, ENOTEMPTY, c_int};
use std::env;
use std::ffi::{OsStr, OsString};
use std::time::{Duration, SystemTime};

use crate::baybridge_adapter::{
    BaybridgeAdapter, BaybridgeContentStore, BaybridgeHashPointerReference,
};
use crate::fuse_file_ext::FuseFileExt;

const TTL: Duration = Duration::ZERO;

mod baybridge_adapter;
mod fuse_file_ext;

#[derive(Debug)]
struct BridgeFS<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore> {
    index_hash: IndexHashT,
    store: StoreT,
    manifest: Manifest,
}

impl<'a> BridgeFS<BaybridgeHashPointerReference<'a>, BaybridgeContentStore<'a>> {
    fn new(adapter: &'a BaybridgeAdapter) -> Self {
        let mut store = adapter.content_store();
        let mut manifest = Manifest::default();

        let root_directory = DirectoryRecord::default();
        let root_hash = store.add_parsed(&Record::Directory(root_directory));

        let initial_index = Index::new(FUSE_ROOT_ID.into(), root_hash);
        let index_hash = store.add_parsed(&initial_index);
        manifest.references.insert((&index_hash).into());

        let mut index_hash = adapter.hash_pointer_reference(index_hash);
        manifest.references.insert(index_hash.get().into());

        BridgeFS {
            index_hash,
            store,
            manifest,
        }
    }
}

impl<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore>
    BridgeFS<IndexHashT, StoreT>
{
    fn get_index(&mut self) -> Index {
        self.store.get_parsed(&self.index_hash.get_typed())
    }

    fn get_record_by_inode(&mut self, inode: INode) -> Option<Record> {
        let index = self.get_index();
        let record_hash = index.get_child_by_inode(&inode)?;
        Some(self.store.get_parsed(record_hash))
    }

    fn add_child(
        &mut self,
        parent_inode: INode,
        name: Filename,
        record: Record,
    ) -> Result<INode, c_int> {
        let mut index = self.get_index();

        let parent = self.get_record_by_inode(parent_inode);
        if parent.is_none() {
            return Err(ENOENT);
        }
        let mut parent = match parent.unwrap() {
            Record::Directory(dir) => dir,
            _ => return Err(ENOTDIR),
        };

        let record_hash = self.store.add_parsed(&record);
        let inode = index.add_child(&mut parent, name, record_hash);

        let parent_hash = self.store.add_parsed(&Record::Directory(parent));
        index.update_child(parent_inode, parent_hash);

        self.index_hash.set_typed(&self.store.add_parsed(&index));
        Ok(inode)
    }
}

impl<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore> Filesystem
    for BridgeFS<IndexHashT, StoreT>
{
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        // TODO: Implement file/directory lookup
        // This is called when the kernel needs to look up a file or directory
        // For now, return "not found" for everything except root
        let index = self.get_index();
        let parent = self.get_record_by_inode(parent.into());
        if parent.is_none() {
            reply.error(ENOENT);
            return;
        }
        let parent = match parent.unwrap() {
            Record::Directory(dir) => dir,
            _ => {
                reply.error(ENOTDIR);
                return;
            }
        };

        let file_data = index.get_child_by_name(&parent, &name.into());
        if file_data.is_none() {
            reply.error(ENOENT);
            return;
        }
        let file_data = file_data.unwrap();
        let record: Record = self.store.get_parsed(&file_data.hash);
        reply.entry(&TTL, &record.attrs(file_data.inode), 0);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        // TODO: Implement getting file attributes
        // This returns metadata about a file or directory
        let index = self.get_index();
        let child_hash = index.get_child_by_inode(&ino.into());
        if child_hash.is_none() {
            reply.error(ENOENT);
            return;
        }
        let record: Record = self.store.get_parsed(child_hash.unwrap());
        reply.attr(&TTL, &record.attrs(ino.into()));
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
        // TODO: Implement file reading
        // This is called when a file's contents need to be read
        let index = self.get_index();
        let file_record = index.get_child_by_inode(&ino.into());
        if file_record.is_none() {
            reply.error(ENOENT);
            return;
        }
        let file_record: FileRecord = match self.store.get_parsed(file_record.unwrap()) {
            Record::File(file_record) => file_record,
            _ => {
                reply.error(EISDIR);
                return;
            }
        };
        let data = self.store.get_parsed(&file_record.content_hash);
        dbg!(&data);

        let data_len = data.len() as i64;
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, data_len as usize);
        if start >= data_len as usize {
            reply.data(&[]);
        } else {
            reply.data(&data.data[start..end]);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        // For now, just return empty directory with . and ..
        let mut entries = vec![
            (FUSE_ROOT_ID, FileType::Directory, OsString::from(".")),
            (FUSE_ROOT_ID, FileType::Directory, OsString::from("..")),
            // TODO: Add your files/directories here
            // Example: (2, FileType::RegularFile, "hello.txt"),
        ];

        let directory = self.get_record_by_inode(ino.into());
        if directory.is_none() {
            reply.error(ENOENT);
            return;
        }
        let directory = match directory.unwrap() {
            Record::Directory(dir) => dir,
            _ => {
                reply.error(ENOTDIR);
                return;
            }
        };

        for entry in directory.list_children() {
            let entry_record = self
                .get_record_by_inode(entry.inode)
                .expect("Directory entry inode should exist");
            entries.push((
                entry.inode.into(),
                entry_record.file_type(),
                entry.name.into(),
            ));
        }

        for (i, (ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(ino, (i + 1) as i64, kind, name) {
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
        let mut index = self.get_index();
        let file_record = index.get_child_by_inode(&ino.into());
        if file_record.is_none() {
            reply.error(ENOENT);
            return;
        }

        let mut file_record: FileRecord = match self.store.get_parsed(file_record.unwrap()) {
            Record::File(file_record) => file_record,
            _ => {
                reply.error(EISDIR);
                return;
            }
        };

        let mut existing_data = self.store.get_parsed(&file_record.content_hash);
        let offset = offset as usize;
        if offset > existing_data.len() {
            existing_data.data.resize(offset, 0);
        }
        if offset + data.len() > existing_data.len() {
            existing_data.data.resize(offset + data.len(), 0);
        }
        existing_data.data[offset..offset + data.len()].copy_from_slice(data);

        file_record.content_hash = self.store.add_parsed(&existing_data);
        file_record.size = existing_data.len() as u64;
        file_record.common_attrs.mtime = SystemTime::now();
        file_record.common_attrs.ctime = SystemTime::now();

        let record = Record::File(file_record);
        let file_record_hash = self.store.add_parsed(&record);
        index.update_child(ino.into(), file_record_hash);
        self.index_hash.set_typed(&self.store.add_parsed(&index));

        dbg!(record);
        reply.written(data.len() as u32);
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
        let empty_data = DataBlock::default();
        let content_hash = self.store.add_parsed(&empty_data);
        let common_attrs = CommonAttrs::builder()
            .perm(get_permissions(mode, umask))
            .uid(req.uid())
            .gid(req.gid())
            .build();
        let file_record = FileRecord::builder()
            .content_hash(content_hash)
            .common_attrs(common_attrs)
            .size(empty_data.len() as u64)
            .build();

        let inode = match self.add_child(
            parent.into(),
            name.into(),
            Record::File(file_record.clone()),
        ) {
            Ok(inode) => inode,
            Err(e) => {
                reply.error(e);
                return;
            }
        };

        let attr = file_record.attrs(inode);
        reply.created(&TTL, &attr, 0, 0, 0);
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
        let common_attrs = CommonAttrs::builder()
            .perm(get_permissions(mode, umask))
            .uid(req.uid())
            .gid(req.gid())
            .build();
        let directory_record = DirectoryRecord::builder()
            .common_attrs(common_attrs)
            .build();

        let inode = match self.add_child(
            parent.into(),
            name.into(),
            Record::Directory(directory_record.clone()),
        ) {
            Ok(inode) => inode,
            Err(e) => {
                reply.error(e);
                return;
            }
        };
        let attr = directory_record.attrs(inode);
        reply.entry(&TTL, &attr, 0);
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

fn main() {
    let mountpoint = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Usage: {} <mountpoint>", env::args().next().unwrap());
            std::process::exit(1);
        }
    };

    let options = vec![
        // MountOption::RO, // Read-only for now
        MountOption::FSName("bridgefs".to_string()),
    ];

    let config = Configuration::default();
    let actions = Actions::new(config);
    let adapter = BaybridgeAdapter::new(actions);
    let bridgefs = BridgeFS::new(&adapter);

    // Mount the filesystem
    if let Err(e) = fuser::mount2(bridgefs, &mountpoint, &options) {
        eprintln!("Failed to mount filesystem: {}", e);
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            eprintln!("Hint: If you need AllowOther, either:");
            eprintln!("  1. Run with sudo, or");
            eprintln!("  2. Add 'user_allow_other' to /etc/fuse.conf");
        }
        std::process::exit(1);
    }
}
