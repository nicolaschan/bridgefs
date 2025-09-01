use crate::{
    data_block::DataBlock,
    file_record::{DirectoryRecord, FileRecord, Record},
    filename::Filename,
    hash_pointer::TypedHashPointer,
    inode::INode,
};

#[derive(Debug)]
pub struct INodeResponse<T, U> {
    pub inner: T,
    pub inode: INode,
    pub source: TypedHashPointer<U>,
}

impl<T, U> INodeResponse<T, U> {
    pub fn new(inner: T, inode: INode, source: TypedHashPointer<U>) -> Self {
        Self {
            inner,
            inode,
            source,
        }
    }

    pub fn convert_inner<V>(self) -> INodeResponse<V, U>
    where
        T: Into<V>,
    {
        INodeResponse {
            inner: self.inner.into(),
            inode: self.inode,
            source: self.source.clone(),
        }
    }

    pub fn swap_inner<V>(self, item: V) -> INodeResponse<V, U> {
        INodeResponse::new(item, self.inode, self.source.clone())
    }
}

impl<T: Clone, U> Clone for INodeResponse<T, U> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            inode: self.inode,
            source: self.source.clone(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FileOperationError {
    NotFound,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    AlreadyExists,
}

#[derive(Debug)]
pub struct ReadFileResponse {
    pub file: INodeResponse<FileRecord, Record>,
    pub datablock: DataBlock,
}

#[derive(Debug)]
pub struct ListDirectoryEntry {
    pub name: Filename,
    pub record: INodeResponse<Record, Record>,
}

#[derive(Debug)]
pub struct ListDirectoryResponse {
    pub directory: INodeResponse<DirectoryRecord, Record>,
    pub entries: Vec<ListDirectoryEntry>,
}
