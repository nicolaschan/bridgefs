use std::time::SystemTime;

use crate::{
    content_store::ContentStore,
    counting_store::CountingStore,
    data_block::DataBlock,
    file_record::{CommonAttrs, DirectoryRecord, FileRecord, Record},
    filename::Filename,
    hash_pointer::{TypedHashPointer, TypedHashPointerReference},
    index::INodeIndex,
    inode::INode,
    manifest::Manifest,
    response::{
        FileOperationError, INodeResponse, ListDirectoryEntry, ListDirectoryResponse,
        ReadFileResponse,
    },
};

#[derive(Debug)]
pub struct BridgeFS<IndexHashT: TypedHashPointerReference<INodeIndex>, StoreT: ContentStore> {
    index_hash: IndexHashT,
    store: CountingStore<StoreT>,
}

impl<IndexHashT: TypedHashPointerReference<INodeIndex>, StoreT: ContentStore>
    BridgeFS<IndexHashT, StoreT>
{
    pub fn new(index_hash: IndexHashT, store: StoreT) -> Self {
        let store = CountingStore::new(store, Manifest::default());
        BridgeFS { index_hash, store }
    }
}

impl<IndexHashT: TypedHashPointerReference<INodeIndex>, StoreT: ContentStore>
    BridgeFS<IndexHashT, StoreT>
{
    fn get_index(&mut self) -> (TypedHashPointer<INodeIndex>, INodeIndex) {
        let index_hash = self.index_hash.get_typed();
        let inode_index = self.store.get_parsed(&index_hash);
        (index_hash, inode_index)
    }

    fn get_record_by_inode(&mut self, inode: INode) -> Option<Record> {
        let (_, index) = self.get_index();
        let record_hash = index.lookup_inode(&inode)?;
        Some(self.store.get_parsed(record_hash))
    }

    fn add_child(
        &mut self,
        parent_inode: INode,
        filename: Filename,
        record: Record,
    ) -> Result<INode, FileOperationError> {
        let mut parent = self.lookup_directory_by_inode(parent_inode)?;
        if parent.inner.children.contains_key(&filename) {
            return Err(FileOperationError::AlreadyExists);
        }

        let (prev_index_hash, mut index) = self.get_index();
        let record_hash = self.store.store_new_content(&record);
        let inode = index.insert_new_inode(record_hash);
        self.index_hash
            .set_typed(&self.store.replace_content(&prev_index_hash, &index));

        parent.inner.insert(filename, inode);
        self.update_index(parent.inode, parent.inner.into());
        Ok(inode)
    }

    fn update_index(&mut self, inode: INode, record: Record) {
        let (prev_index_hash, mut index) = self.get_index();
        let prev_inode_hash = index
            .lookup_inode(&inode)
            .expect("INode should exist prior to update");
        let new_inode_hash = self.store.replace_content(prev_inode_hash, &record);

        index.update_inode(inode, new_inode_hash);
        let new_index_hash = self.store.replace_content(&prev_index_hash, &index);
        self.index_hash.set_typed(&new_index_hash);
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
        let parent = self.lookup_directory_by_inode(parent)?;
        match parent.inner.get(name) {
            Some(&inode) => self.lookup_record_by_inode(inode),
            None => Err(FileOperationError::NotFound),
        }
    }

    fn lookup_directory_by_name(
        &mut self,
        parent: INode,
        name: &Filename,
    ) -> Result<INodeResponse<DirectoryRecord>, FileOperationError> {
        let record = self.lookup_record_by_name(parent, name)?;
        match record.inner {
            Record::Directory(directory) => Ok(INodeResponse::new(directory, record.inode)),
            _ => Err(FileOperationError::NotADirectory),
        }
    }

    fn lookup_file_by_name(
        &mut self,
        parent: INode,
        name: &Filename,
    ) -> Result<INodeResponse<FileRecord>, FileOperationError> {
        let record = self.lookup_record_by_name(parent, name)?;
        match record.inner {
            Record::File(file) => Ok(INodeResponse::new(file, record.inode)),
            _ => Err(FileOperationError::IsADirectory),
        }
    }

    pub fn read_file_data_by_inode(
        &mut self,
        inode: INode,
        offset: usize,
        size: usize,
    ) -> Result<ReadFileResponse, FileOperationError> {
        let file = self.lookup_file_by_inode(inode)?;
        let data = self.store.get_parsed(&file.inner.content_hash);

        let data_len = data.len();
        let start = offset;
        let end = std::cmp::min(start + size, data_len);
        if start >= data_len {
            return Ok(ReadFileResponse {
                file,
                datablock: DataBlock::default(),
            });
        }
        Ok(ReadFileResponse {
            file,
            datablock: DataBlock {
                data: data.data[start..end].to_vec(),
            },
        })
    }

    pub fn create_file(
        &mut self,
        parent: INode,
        name: Filename,
        attributes: CommonAttrs,
    ) -> Result<INodeResponse<FileRecord>, FileOperationError> {
        let empty_data = DataBlock::default();
        let content_hash = self.store.store_new_content(&empty_data);
        let file_record = FileRecord::builder()
            .content_hash(content_hash)
            .common_attrs(attributes)
            .size(empty_data.len() as u64)
            .build();
        let inode = self.add_child(parent, name, Record::File(file_record.clone()))?;
        Ok(INodeResponse::new(file_record, inode))
    }

    pub fn create_directory(
        &mut self,
        parent: INode,
        name: Filename,
        attributes: CommonAttrs,
    ) -> Result<INodeResponse<DirectoryRecord>, FileOperationError> {
        let directory_record = DirectoryRecord::builder()
            .common_attrs(attributes)
            .parent(parent)
            .build();
        let inode = self.add_child(parent, name, Record::Directory(directory_record.clone()))?;
        Ok(INodeResponse::new(directory_record, inode))
    }

    pub fn write_to_file(
        &mut self,
        inode: INode,
        offset: usize,
        data: &[u8],
    ) -> Result<usize, FileOperationError> {
        // TODO: support sparse files and writing without needing to read existing data
        let mut existing_data = self.read_file_data_by_inode(inode, 0, usize::MAX)?;
        if offset > existing_data.datablock.len() {
            existing_data.datablock.data.resize(offset, 0);
        }
        if offset + data.len() > existing_data.datablock.len() {
            existing_data.datablock.data.resize(offset + data.len(), 0);
        }
        existing_data.datablock.data[offset..offset + data.len()].copy_from_slice(data);

        existing_data.file.inner.content_hash =
            self.store.store_new_content(&existing_data.datablock);
        existing_data.file.inner.size = existing_data.datablock.len() as u64;
        existing_data.file.inner.common_attrs.mtime = SystemTime::now();
        existing_data.file.inner.common_attrs.ctime = SystemTime::now();

        let new_record = Record::File(existing_data.file.inner);
        self.update_index(inode, new_record);
        Ok(data.len())
    }

    pub fn list_directory_by_inode(
        &mut self,
        inode: INode,
    ) -> Result<ListDirectoryResponse, FileOperationError> {
        let directory = self.lookup_directory_by_inode(inode)?;

        let mut entries = Vec::new();

        for entry in directory.inner.list_children() {
            let record = self.lookup_record_by_inode(entry.inode)?;
            entries.push(ListDirectoryEntry {
                name: entry.name,
                record,
            });
        }

        entries.push(ListDirectoryEntry {
            name: ".".into(),
            record: directory.clone().convert_inner(),
        });

        let parent = self.lookup_record_by_inode(directory.inner.parent)?;
        entries.push(ListDirectoryEntry {
            name: "..".into(),
            record: parent,
        });

        Ok(ListDirectoryResponse { directory, entries })
    }

    pub fn remove_directory_by_name(
        &mut self,
        parent: INode,
        name: &Filename,
    ) -> Result<(), FileOperationError> {
        let target = self.lookup_directory_by_name(parent, name)?;
        if !target.inner.children.is_empty() {
            return Err(FileOperationError::DirectoryNotEmpty);
        }

        let mut parent = self.lookup_directory_by_inode(parent)?;
        parent.inner.remove(name);
        self.update_index(parent.inode, parent.inner.into());
        Ok(())
    }

    pub fn remove_file_by_name(
        &mut self,
        parent: INode,
        name: &Filename,
    ) -> Result<(), FileOperationError> {
        self.lookup_file_by_name(parent, name)?; // Ensure target exists and is a file

        let mut parent = self.lookup_directory_by_inode(parent)?;
        parent.inner.remove(name);
        self.update_index(parent.inode, parent.inner.into());
        Ok(())
    }

    pub fn update_attributes_by_inode(
        &mut self,
        inode: INode,
        attributes: CommonAttrs,
    ) -> Result<INodeResponse<Record>, FileOperationError> {
        let mut record = self.lookup_record_by_inode(inode)?;
        record.inner.set_attrs(attributes);
        self.update_index(inode, record.inner.clone());
        Ok(INodeResponse::new(record.inner, inode))
    }
}
