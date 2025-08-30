use bridgefs_core::{
    content_store::ParsingContentStoreExt,
    file_record::{DirectoryRecord, Record},
    hash_pointer::TypedHashPointer,
    index::Index,
};
use fuser::FUSE_ROOT_ID;

pub trait FuseStoreExt {
    fn empty_root_dir(&mut self) -> TypedHashPointer<Index>;
}

impl<T: ParsingContentStoreExt> FuseStoreExt for T {
    fn empty_root_dir(&mut self) -> TypedHashPointer<Index> {
        let root_directory = DirectoryRecord::default();
        let root_hash = self.add_parsed(&Record::Directory(root_directory));

        let initial_index = Index::new(FUSE_ROOT_ID.into(), root_hash);
        self.add_parsed(&initial_index)
    }
}
