use crate::{
    data_block::DataBlock,
    file_record::{DirectoryRecord, FileRecord, Record},
    filename::Filename,
    inode::INode,
};

#[derive(Debug)]
pub struct INodeResponse<T> {
    pub inner: T,
    pub inode: INode,
}

impl<T> INodeResponse<T> {
    pub fn new(inner: T, inode: INode) -> Self {
        Self { inner, inode }
    }

    pub fn convert_inner<U>(self) -> INodeResponse<U>
    where
        T: Into<U>,
    {
        INodeResponse {
            inner: self.inner.into(),
            inode: self.inode,
        }
    }
}

impl<T: Clone> Clone for INodeResponse<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            inode: self.inode,
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
    pub file: INodeResponse<FileRecord>,
    pub datablock: DataBlock,
}

#[derive(Debug)]
pub struct ListDirectoryEntry {
    pub name: Filename,
    pub record: INodeResponse<Record>,
}

#[derive(Debug)]
pub struct ListDirectoryResponse {
    pub directory: INodeResponse<DirectoryRecord>,
    pub entries: Vec<ListDirectoryEntry>,
}
