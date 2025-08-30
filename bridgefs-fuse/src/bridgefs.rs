use std::time::SystemTime;

use bridgefs_core::{
    content_store::{ContentStore, ParsingContentStoreExt},
    data_block::DataBlock,
    file_record::{CommonAttrs, DirectoryRecord, FileRecord, Record},
    filename::Filename,
    hash_pointer::TypedHashPointerReference,
    index::Index,
    inode::INode,
    response::{
        FileOperationError, INodeResponse, ListDirectoryEntry, ListDirectoryResponse,
        ReadFileResponse,
    },
};

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

    fn update_index(&mut self, inode: INode, record: Record) {
        let mut index = self.get_index();
        let record_hash = self.store.add_parsed(&record);
        index.update_child(inode, record_hash);
        self.index_hash.set_typed(&self.store.add_parsed(&index));
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
        } else {
            return Ok(ReadFileResponse {
                file,
                datablock: DataBlock {
                    data: data.data[start..end].to_vec(),
                },
            });
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
        let directory_record = DirectoryRecord::builder()
            .common_attrs(attributes)
            .parent(parent)
            .build();
        let inode = self.add_child(
            parent.into(),
            name.into(),
            Record::Directory(directory_record.clone()),
        )?;
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
        let offset = offset;
        if offset > existing_data.datablock.len() {
            existing_data.datablock.data.resize(offset, 0);
        }
        if offset + data.len() > existing_data.datablock.len() {
            existing_data.datablock.data.resize(offset + data.len(), 0);
        }
        existing_data.datablock.data[offset..offset + data.len()].copy_from_slice(data);

        existing_data.file.inner.content_hash = self.store.add_parsed(&existing_data.datablock);
        existing_data.file.inner.size = existing_data.datablock.len() as u64;
        existing_data.file.inner.common_attrs.mtime = SystemTime::now();
        existing_data.file.inner.common_attrs.ctime = SystemTime::now();

        let new_record = Record::File(existing_data.file.inner);
        self.update_index(inode.into(), new_record);
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
                name: entry.name.into(),
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
}
