use bridgefs_core::{
    content_store::{ContentStore, ParsingContentStoreExt},
    data_block::DataBlock,
    file_record::{CommonAttrs, DirectoryRecord, FileRecord, Record},
    filename::Filename,
    hash_pointer::TypedHashPointerReference,
    index::Index,
    inode::INode,
    response::{FileOperationError, INodeResponse},
};
use libc::{ENOENT, ENOTDIR, c_int};

#[derive(Debug)]
pub struct BridgeFS<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore> {
    // TODO: make private
    pub index_hash: IndexHashT,
    // TODO: make private
    pub store: StoreT,
}

impl<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore>
    BridgeFS<IndexHashT, StoreT>
{
    pub fn new(index_hash: IndexHashT, store: StoreT) -> Self {
        BridgeFS { index_hash, store }
    }
}

impl<IndexHashT: TypedHashPointerReference<Index>, StoreT: ContentStore>
    BridgeFS<IndexHashT, StoreT>
{
    // TODO: make private
    pub fn get_index(&mut self) -> Index {
        self.store.get_parsed(&self.index_hash.get_typed())
    }

    // TODO: make private
    pub fn get_record_by_inode(&mut self, inode: INode) -> Option<Record> {
        let index = self.get_index();
        let record_hash = index.get_child_by_inode(&inode)?;
        Some(self.store.get_parsed(record_hash))
    }

    // TODO: make private
    pub fn add_child(
        &mut self,
        parent_inode: INode,
        name: Filename,
        record: Record,
    ) -> Result<INode, FileOperationError> {
        let mut index = self.get_index();

        let parent = self.get_record_by_inode(parent_inode);
        if parent.is_none() {
            return Err(FileOperationError::NotFound);
        }
        let mut parent = match parent.unwrap() {
            Record::Directory(dir) => dir,
            _ => return Err(FileOperationError::NotADirectory),
        };

        let record_hash = self.store.add_parsed(&record);
        let inode = index.add_child(&mut parent, name, record_hash);

        let parent_hash = self.store.add_parsed(&Record::Directory(parent));
        index.update_child(parent_inode, parent_hash);

        self.index_hash.set_typed(&self.store.add_parsed(&index));
        Ok(inode)
    }

    pub fn lookup_record_by_inode(
        &mut self,
        inode: INode,
    ) -> Result<INodeResponse<Record>, FileOperationError> {
        let record = self.get_record_by_inode(inode);
        if record.is_none() {
            return Err(FileOperationError::NotFound);
        }
        Ok(INodeResponse {
            inner: record.unwrap(),
            inode,
        })
    }

    pub fn lookup_file_by_inode(
        &mut self,
        inode: INode,
    ) -> Result<INodeResponse<FileRecord>, FileOperationError> {
        let record = self.lookup_record_by_inode(inode)?;
        match record.inner {
            Record::File(file) => Ok(INodeResponse::new(file, record.inode)),
            _ => Err(FileOperationError::IsADirectory),
        }
    }

    fn lookup_directory_by_inode(
        &mut self,
        inode: INode,
    ) -> Result<INodeResponse<DirectoryRecord>, FileOperationError> {
        let record = self.lookup_record_by_inode(inode)?;
        match record.inner {
            Record::Directory(directory) => Ok(INodeResponse::new(directory, record.inode)),
            _ => Err(FileOperationError::NotADirectory),
        }
    }

    pub fn lookup_record_by_name(
        &mut self,
        parent: INode,
        name: &Filename,
    ) -> Result<INodeResponse<Record>, FileOperationError> {
        let index = self.get_index();
        let parent = self.lookup_directory_by_inode(parent)?;

        let file_data = index.get_child_by_name(&parent.inner, name);
        if file_data.is_none() {
            return Err(FileOperationError::NotFound);
        }
        let file_data = file_data.unwrap();
        let record = self.store.get_parsed(&file_data.hash);
        Ok(INodeResponse::new(record, file_data.inode))
    }

    pub fn read_file_data_by_inode(
        &mut self,
        inode: INode,
        offset: usize,
        size: usize,
    ) -> Result<Vec<u8>, FileOperationError> {
        let file_record = self.lookup_file_by_inode(inode)?;
        let data = self.store.get_parsed(&file_record.inner.content_hash);

        let data_len = data.len();
        let start = offset;
        let end = std::cmp::min(start + size, data_len);
        if start >= data_len {
            return Ok(vec![]);
        } else {
            return Ok(data.data[start..end].to_vec());
        }
    }

    pub fn create_file(
        &mut self,
        parent: INode,
        name: Filename,
        attributes: CommonAttrs,
    ) -> Result<INodeResponse<FileRecord>, FileOperationError> {
        let empty_data = DataBlock::default();
        let content_hash = self.store.add_parsed(&empty_data);
        let file_record = FileRecord::builder()
            .content_hash(content_hash)
            .common_attrs(attributes)
            .size(empty_data.len() as u64)
            .build();
        let inode = self.add_child(
            parent.into(),
            name.into(),
            Record::File(file_record.clone()),
        )?;
        Ok(INodeResponse::new(file_record, inode))
    }

    pub fn create_directory(
        &mut self,
        parent: INode,
        name: Filename,
        attributes: CommonAttrs,
    ) -> Result<INodeResponse<DirectoryRecord>, FileOperationError> {
        let directory_record = DirectoryRecord::builder().common_attrs(attributes).build();
        let inode = self.add_child(
            parent.into(),
            name.into(),
            Record::Directory(directory_record.clone()),
        )?;
        Ok(INodeResponse::new(directory_record, inode))
    }
}
