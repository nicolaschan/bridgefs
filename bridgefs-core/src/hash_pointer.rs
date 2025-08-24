use std::{fmt::Debug, marker::PhantomData};

use base64::{Engine, engine::general_purpose};
use bincode::{Decode, Encode};

#[derive(Encode, Decode, Hash, PartialEq, Eq, Clone)]
pub struct HashPointer {
    bytes: [u8; 32],
}

impl From<blake3::Hash> for HashPointer {
    fn from(blake3_hash: blake3::Hash) -> Self {
        HashPointer {
            bytes: *blake3_hash.as_bytes(),
        }
    }
}

impl<T> From<&TypedHashPointer<T>> for HashPointer {
    fn from(typed_hash_pointer: &TypedHashPointer<T>) -> Self {
        typed_hash_pointer.hash_pointer.clone()
    }
}

impl Debug for HashPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HashPointer({})",
            general_purpose::STANDARD.encode(self.bytes)
        )
    }
}

#[derive(Encode, Decode, Hash, PartialEq, Eq, Clone, Debug)]
pub struct TypedHashPointer<T> {
    hash_pointer: HashPointer,
    _marker: PhantomData<T>,
}

impl<T> TypedHashPointer<T> {
    pub fn new(hash_pointer: HashPointer) -> Self {
        Self {
            hash_pointer,
            _marker: PhantomData,
        }
    }
}
