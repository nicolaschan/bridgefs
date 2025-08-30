use crate::inode::INode;

#[derive(Debug)]
pub struct INodeResponse<T> {
    pub inner: T,
    pub inode: INode,
}

impl<T> INodeResponse<T> {
    pub fn new(inner: T, inode: INode) -> Self {
        Self { inner, inode }
    }
}

#[derive(Debug, PartialEq)]
pub enum FileOperationError {
    NotFound,
    NotADirectory,
    IsADirectory,
}
