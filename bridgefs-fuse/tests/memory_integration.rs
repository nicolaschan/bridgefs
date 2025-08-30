use std::ffi::OsStr;

use bridgefs_core::{
    content_store::InMemoryContentStore, hash_pointer::InMemoryHashPointerReference,
    response::FileOperationError,
};
use bridgefs_fuse::{bridgefs::BridgeFS, fuse_store_ext::FuseStoreExt};
use fuser::FUSE_ROOT_ID;

fn empty_in_memory_bridgefs() -> BridgeFS<InMemoryHashPointerReference, InMemoryContentStore> {
    let mut store = InMemoryContentStore::default();
    let initial_index_hash = store.empty_root_dir();
    let pointer = InMemoryHashPointerReference::new(initial_index_hash.into());
    BridgeFS::new(pointer, store)
}

#[test]
fn test_lookup_by_name_missing_file() {
    let mut bridgefs = empty_in_memory_bridgefs();
    let filename: &OsStr = "nonexistent".as_ref();
    let record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &filename.into());

    assert!(record.is_err());
    assert_eq!(record.unwrap_err(), FileOperationError::NotFound);
}

#[test]
fn test_lookup_by_name_in_missing_parent_directory() {
    let mut bridgefs = empty_in_memory_bridgefs();
    let filename: &OsStr = "file".as_ref();
    let record = bridgefs.lookup_record_by_name(42.into(), &filename.into());

    assert!(record.is_err());
    assert_eq!(record.unwrap_err(), FileOperationError::NotFound);
}

#[test]
fn test_lookup_by_inode_missing_file() {
    let mut bridgefs = empty_in_memory_bridgefs();
    let record = bridgefs.lookup_record_by_inode(42.into());

    assert!(record.is_err());
    assert_eq!(record.unwrap_err(), FileOperationError::NotFound);
}
