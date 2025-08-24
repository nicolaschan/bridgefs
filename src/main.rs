use fuser::{
    FUSE_ROOT_ID, FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyWrite, Request, TimeOrNow,
};
use libc::ENOENT;
use std::env;
use std::ffi::{OsStr, OsString};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::content_store::{ContentStore, ParsingContentStore};
use crate::data_block::DataBlock;
use crate::file_record::FileRecord;
use crate::index::Index;

const TTL: Duration = Duration::from_secs(0);

mod content_store;
mod data_block;
mod file_record;
mod filename;
mod hash_pointer;
mod index;
mod inode;

#[derive(Debug)]
struct BridgeFS {
    index_hash: hash_pointer::HashPointer,
    store: ParsingContentStore<content_store::InMemoryContentStore>,
}

impl BridgeFS {
    fn new() -> Self {
        let initial_index = index::Index::default();
        let initial_index_bytes =
            bincode::encode_to_vec(&initial_index, bincode::config::standard())
                .expect("Failed to encode initial index");
        let store = content_store::InMemoryContentStore::new();
        let mut store = ParsingContentStore::new(store);
        let index_hash = store.add_content(&initial_index_bytes);
        BridgeFS { index_hash, store }
    }

    fn get_index(&mut self) -> Index {
        self.store.get_parsed(&self.index_hash)
    }
}

impl Filesystem for BridgeFS {
    fn lookup(&mut self, _req: &Request, _parent: u64, name: &OsStr, reply: ReplyEntry) {
        // TODO: Implement file/directory lookup
        // This is called when the kernel needs to look up a file or directory
        // For now, return "not found" for everything except root
        let index = self.get_index();
        let file_data = index.get_child_by_name(&name.into());
        if file_data.is_none() {
            reply.error(ENOENT);
            return;
        }
        let file_data = file_data.unwrap();
        let file_record: FileRecord = self.store.get_parsed(&file_data.hash);
        reply.entry(&TTL, &file_record.attr(file_data.inode), 0);
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        // TODO: Implement getting file attributes
        // This returns metadata about a file or directory
        if ino == FUSE_ROOT_ID {
            // Root directory attributes
            let attr = FileAttr {
                ino: FUSE_ROOT_ID,
                size: 0,
                blocks: 0,
                atime: UNIX_EPOCH,
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 501, // Change to appropriate user ID
                gid: 20,  // Change to appropriate group ID
                rdev: 0,
                flags: 0,
                blksize: 512,
            };
            reply.attr(&TTL, &attr);
            return;
        }

        let index = self.get_index();
        let child_hash = index.get_child_by_inode(&ino.into());
        if child_hash.is_none() {
            reply.error(ENOENT);
            return;
        }
        let file_record: FileRecord = self.store.get_parsed(child_hash.unwrap());
        reply.attr(&TTL, &file_record.attr(ino.into()));
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
        let file_record: FileRecord = self.store.get_parsed(file_record.unwrap());
        let data = self.store.get_content(&file_record.content_hash);
        dbg!(&data);

        let data_len = data.len() as i64;
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, data_len as usize);
        if start >= data_len as usize {
            reply.data(&[]);
        } else {
            reply.data(&data[start..end]);
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
        // TODO: Implement directory listing
        // This is called to list the contents of a directory
        if ino != FUSE_ROOT_ID {
            reply.error(ENOENT);
            return;
        }

        // For now, just return empty directory with . and ..
        let mut entries = vec![
            (FUSE_ROOT_ID, FileType::Directory, OsString::from(".")),
            (FUSE_ROOT_ID, FileType::Directory, OsString::from("..")),
            // TODO: Add your files/directories here
            // Example: (2, FileType::RegularFile, "hello.txt"),
        ];

        for entry in self.get_index().list_children() {
            entries.push((entry.inode.into(), FileType::RegularFile, entry.name.into()));
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

        let mut file_record: FileRecord = self.store.get_parsed(file_record.unwrap());

        let mut existing_data = self.store.get_content(&file_record.content_hash);
        let offset = offset as usize;
        if offset > existing_data.len() {
            existing_data.resize(offset, 0);
        }
        if offset + data.len() > existing_data.len() {
            existing_data.resize(offset + data.len(), 0);
        }
        existing_data[offset..offset + data.len()].copy_from_slice(data);

        file_record.content_hash = self.store.add_content(&existing_data);
        file_record.size = existing_data.len() as u64;
        file_record.mtime = SystemTime::now();
        file_record.ctime = SystemTime::now();
        let file_record_hash = self.store.add_parsed(&file_record);
        index.update_child(ino.into(), file_record_hash);
        self.index_hash = self.store.add_parsed(&index);

        dbg!(file_record);
        reply.written(data.len() as u32);
    }

    fn create(
        &mut self,
        req: &Request,
        _parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let mut index = self.get_index();

        let empty_data = DataBlock::default();
        let content_hash = self.store.add_parsed(&empty_data);
        let perm = (mode & 0o7777 & !umask) as u16;
        let file_record = FileRecord::builder()
            .content_hash(content_hash)
            .perm(perm)
            .uid(req.uid())
            .gid(req.gid())
            .build();
        let file_record_hash = self.store.add_parsed(&file_record);

        let inode = index.add_child(name.into(), file_record_hash);
        let attr = file_record.attr(inode);
        self.index_hash = self.store.add_parsed(&index);
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
        let mut file_record: FileRecord = self.store.get_parsed(child_hash.unwrap());
        if let Some(mode) = mode {
            file_record.perm = mode as u16;
        }

        if let Some(atime) = atime {
            file_record.atime = match atime {
                TimeOrNow::SpecificTime(t) => t,
                TimeOrNow::Now => SystemTime::now(),
            };
        }

        if let Some(mtime) = mtime {
            file_record.mtime = match mtime {
                TimeOrNow::SpecificTime(t) => t,
                TimeOrNow::Now => SystemTime::now(),
            };
        }

        if let Some(ctime) = ctime {
            file_record.ctime = ctime;
        }

        if let Some(crtime) = crtime {
            file_record.crtime = crtime;
        }

        let new_file_record_hash = self.store.add_parsed(&file_record);
        index.update_child(ino.into(), new_file_record_hash);
        self.index_hash = self.store.add_parsed(&index);

        let attr = file_record.attr(ino.into());
        reply.attr(&TTL, &attr);
    }

    fn mkdir(
        &mut self,
        _req: &Request,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _reply: ReplyEntry,
    ) {
        todo!()
    }

    fn unlink(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _reply: ReplyEmpty) {
        todo!()
    }

    fn rmdir(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _reply: ReplyEmpty) {
        todo!()
    }
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

    // Mount the filesystem
    if let Err(e) = fuser::mount2(BridgeFS::new(), &mountpoint, &options) {
        eprintln!("Failed to mount filesystem: {}", e);
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            eprintln!("Hint: If you need AllowOther, either:");
            eprintln!("  1. Run with sudo, or");
            eprintln!("  2. Add 'user_allow_other' to /etc/fuse.conf");
        }
        std::process::exit(1);
    }
}
