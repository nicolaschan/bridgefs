use std::ffi::OsStr;

use bridgefs_core::{
    bridgefs::BridgeFS, content_store::InMemoryContentStore, file_record::CommonAttrs,
    hash_pointer::InMemoryHashPointerReference, response::FileOperationError,
};
use bridgefs_fuse::fuse_store_ext::FuseStoreExt;
use fuser::FUSE_ROOT_ID;

static EMPTY_FILENAME: &str = "empty_file";
static FILENAME: &str = "file";
static DIRNAME: &str = "dir";
static FILE_UNDER_DIR: &str = "file_under_dir";
static EMPTY_DIRNAME: &str = "empty_dir";

fn empty_in_memory_bridgefs() -> BridgeFS<InMemoryHashPointerReference, InMemoryContentStore> {
    let mut store = InMemoryContentStore::default();
    let initial_index_hash = store.empty_root_dir();
    let pointer = InMemoryHashPointerReference::new(initial_index_hash.into());
    BridgeFS::new(pointer, store)
}

fn in_memory_bridgefs() -> BridgeFS<InMemoryHashPointerReference, InMemoryContentStore> {
    let mut bridgefs = empty_in_memory_bridgefs();
    bridgefs
        .create_file(
            FUSE_ROOT_ID.into(),
            EMPTY_FILENAME.into(),
            CommonAttrs::default(),
        )
        .expect("Failed to create file");

    let content_file = bridgefs
        .create_file(FUSE_ROOT_ID.into(), FILENAME.into(), CommonAttrs::default())
        .expect("Failed to create file");
    let data = b"Hello, BridgeFS!";
    bridgefs
        .write_to_file(content_file.inode, 0, data)
        .expect("Failed to write data");

    let dir = bridgefs
        .create_directory(FUSE_ROOT_ID.into(), DIRNAME.into(), CommonAttrs::default())
        .expect("Failed to create directory");
    let file_under_dir = bridgefs
        .create_file(dir.inode, FILE_UNDER_DIR.into(), CommonAttrs::default())
        .expect("Failed to create file under directory");
    bridgefs
        .write_to_file(file_under_dir.inode, 0, b"File under directory")
        .expect("Failed to write data to file under directory");

    bridgefs
        .create_directory(
            FUSE_ROOT_ID.into(),
            EMPTY_DIRNAME.into(),
            CommonAttrs::default(),
        )
        .expect("Failed to create empty directory");
    bridgefs
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

#[test]
fn test_read_by_inode_missing_file() {
    let mut bridgefs = empty_in_memory_bridgefs();
    let read_result = bridgefs.read_file_data_by_inode(42.into(), 0, 1024);

    assert!(read_result.is_err());
    assert_eq!(read_result.unwrap_err(), FileOperationError::NotFound);
}

#[test]
fn test_lookup_by_name_existing_file() {
    let mut bridgefs = in_memory_bridgefs();
    let record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &EMPTY_FILENAME.into());
    assert!(record.is_ok());
}

#[test]
fn test_read_by_inode_empty_file() {
    let mut bridgefs = in_memory_bridgefs();
    let record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &EMPTY_FILENAME.into());
    assert!(record.is_ok());
    let inode = record.unwrap().inode;

    let read_result = bridgefs.read_file_data_by_inode(inode, 0, 1024);
    assert!(read_result.is_ok());
    let data = read_result.unwrap();
    assert!(data.datablock.is_empty());
}

#[test]
fn test_read_by_inode_file_with_content() {
    let mut bridgefs = in_memory_bridgefs();
    let record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(record.is_ok());
    let inode = record.unwrap().inode;

    let read_result = bridgefs.read_file_data_by_inode(inode, 0, 1024);
    assert!(read_result.is_ok());
    let data = read_result.unwrap();
    assert_eq!(data.datablock.data, b"Hello, BridgeFS!");
}

#[test]
fn test_read_file_under_directory() {
    let mut bridgefs = in_memory_bridgefs();
    let dir_record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &DIRNAME.into());
    assert!(dir_record.is_ok());
    let dir_inode = dir_record.unwrap().inode;

    let file_record = bridgefs.lookup_record_by_name(dir_inode, &FILE_UNDER_DIR.into());
    assert!(file_record.is_ok());
    let file_inode = file_record.unwrap().inode;

    let read_result = bridgefs.read_file_data_by_inode(file_inode, 0, 1024);
    assert!(read_result.is_ok());
    let data = read_result.unwrap();
    assert_eq!(data.datablock.data, b"File under directory");
}

#[test]
fn test_list_root_directory() {
    let mut bridgefs = in_memory_bridgefs();
    let entries = bridgefs.list_directory_by_inode(FUSE_ROOT_ID.into());
    assert!(entries.is_ok());
    let entries = entries.unwrap();
    let names: Vec<String> = entries.entries.into_iter().map(|e| e.name.into()).collect();

    assert!(names.contains(&".".to_string()));
    assert!(names.contains(&"..".to_string()));
    assert!(names.contains(&EMPTY_FILENAME.to_string()));
    assert!(names.contains(&FILENAME.to_string()));
    assert!(names.contains(&DIRNAME.to_string()));
    assert!(names.contains(&EMPTY_DIRNAME.to_string()));
    assert_eq!(names.len(), 6);
}

#[test]
fn test_list_subdirectory() {
    let mut bridgefs = in_memory_bridgefs();
    let dir_record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &DIRNAME.into());
    assert!(dir_record.is_ok());
    let dir_inode = dir_record.unwrap().inode;

    let entries = bridgefs.list_directory_by_inode(dir_inode);
    assert!(entries.is_ok());
    let entries = entries.unwrap();
    let names: Vec<String> = entries.entries.into_iter().map(|e| e.name.into()).collect();

    assert!(names.contains(&".".to_string()));
    assert!(names.contains(&"..".to_string()));
    assert!(names.contains(&FILE_UNDER_DIR.to_string()));
    assert_eq!(names.len(), 3);
}

#[test]
fn test_remove_directory_non_empty() {
    let mut bridgefs = in_memory_bridgefs();
    let remove_result = bridgefs.remove_directory_by_name(FUSE_ROOT_ID.into(), &DIRNAME.into());
    assert!(remove_result.is_err());
    assert_eq!(
        remove_result.unwrap_err(),
        FileOperationError::DirectoryNotEmpty
    );

    // Ensure the directory still exists
    let dir_record_after = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &DIRNAME.into());
    assert!(dir_record_after.is_ok());
}

#[test]
fn test_remove_directory_empty() {
    let mut bridgefs = in_memory_bridgefs();
    let remove_result =
        bridgefs.remove_directory_by_name(FUSE_ROOT_ID.into(), &EMPTY_DIRNAME.into());
    assert!(remove_result.is_ok());

    // Ensure the directory is removed
    let dir_record_after =
        bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &EMPTY_DIRNAME.into());
    assert!(dir_record_after.is_err());
    assert_eq!(dir_record_after.unwrap_err(), FileOperationError::NotFound);
}

#[test]
fn test_remove_directory_on_file() {
    let mut bridgefs = in_memory_bridgefs();
    let remove_result = bridgefs.remove_directory_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(remove_result.is_err());
    assert_eq!(
        remove_result.unwrap_err(),
        FileOperationError::NotADirectory
    );

    // Ensure the file still exists
    let file_record_after = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(file_record_after.is_ok());
}

#[test]
fn test_remove_file() {
    let mut bridgefs = in_memory_bridgefs();
    let remove_result = bridgefs.remove_file_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(remove_result.is_ok());

    // Ensure the file is removed
    let file_record_after = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(file_record_after.is_err());
    assert_eq!(file_record_after.unwrap_err(), FileOperationError::NotFound);
}

#[test]
fn test_remove_file_on_directory() {
    let mut bridgefs = in_memory_bridgefs();
    let remove_result = bridgefs.remove_file_by_name(FUSE_ROOT_ID.into(), &DIRNAME.into());
    assert!(remove_result.is_err());
    assert_eq!(remove_result.unwrap_err(), FileOperationError::IsADirectory);

    // Ensure the directory still exists
    let dir_record_after = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &DIRNAME.into());
    assert!(dir_record_after.is_ok());
}

#[test]
fn test_update_attributes_on_file() {
    let mut bridgefs = in_memory_bridgefs();
    let record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(record.is_ok());
    let inode = record.unwrap().inode;

    let new_attrs = CommonAttrs {
        uid: 1000,
        gid: 1000,
        perm: 0o644,
        ..Default::default()
    };
    let update_result = bridgefs.update_attributes_by_inode(inode, new_attrs);
    assert!(update_result.is_ok());

    let updated_record = bridgefs.lookup_record_by_inode(inode);
    assert!(updated_record.is_ok());
    let updated_record = updated_record.unwrap().inner;
    let updated_attrs = updated_record.common_attrs();
    assert_eq!(updated_attrs.uid, 1000);
    assert_eq!(updated_attrs.gid, 1000);
    assert_eq!(updated_attrs.perm, 0o644);
}

#[test]
fn test_read_after_update_attributes() {
    let mut bridgefs = in_memory_bridgefs();
    let record = bridgefs.lookup_record_by_name(FUSE_ROOT_ID.into(), &FILENAME.into());
    assert!(record.is_ok());
    let inode = record.unwrap().inode;

    let new_attrs = CommonAttrs {
        uid: 1000,
        gid: 1000,
        perm: 0o644,
        ..Default::default()
    };
    let update_result = bridgefs.update_attributes_by_inode(inode, new_attrs);
    assert!(update_result.is_ok());

    let read_result = bridgefs.read_file_data_by_inode(inode, 0, 1024);
    assert!(read_result.is_ok());
    let data = read_result.unwrap();
    assert_eq!(data.datablock.data, b"Hello, BridgeFS!");
}

#[test]
fn test_create_file_already_exists() {
    let mut bridgefs = in_memory_bridgefs();
    let result = bridgefs.create_file(
        FUSE_ROOT_ID.into(),
        EMPTY_FILENAME.into(),
        CommonAttrs::default(),
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), FileOperationError::AlreadyExists);
}

#[test]
fn test_create_directory_already_exists() {
    let mut bridgefs = in_memory_bridgefs();
    let result =
        bridgefs.create_directory(FUSE_ROOT_ID.into(), DIRNAME.into(), CommonAttrs::default());
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), FileOperationError::AlreadyExists);
}
