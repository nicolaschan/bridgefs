use std::ffi::OsStr;

use bridgefs_core::{
    content_store::InMemoryContentStore, hash_pointer::InMemoryHashPointerReference,
};
use bridgefs_fuse::{BridgeFS, fuse_store_ext::FuseStoreExt};
use fuser::FUSE_ROOT_ID;

fn empty_in_memory_bridgefs() -> BridgeFS<InMemoryHashPointerReference, InMemoryContentStore> {
    let mut store = InMemoryContentStore::default();
    let initial_index_hash = store.empty_root_dir();
    let pointer = InMemoryHashPointerReference::new(initial_index_hash.into());
    BridgeFS::new(pointer, store)
}

#[test]
fn test_lookup_missing_file() {
    let mut bridgefs = empty_in_memory_bridgefs();
    let filename: &OsStr = "nonexistent".as_ref();
    let attrs = bridgefs.lookup_record(FUSE_ROOT_ID.into(), &filename.into());

    assert!(attrs.is_err());
    assert_eq!(attrs.unwrap_err(), libc::ENOENT);
}
