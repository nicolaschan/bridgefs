use std::{fmt::Debug, marker::PhantomData};

use base64::{Engine, engine::general_purpose};
use bincode::{Decode, Encode};

#[derive(Encode, Decode, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
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

impl From<&HashPointer> for blake3::Hash {
    fn from(hash_pointer: &HashPointer) -> Self {
        blake3::Hash::from_bytes(hash_pointer.bytes)
    }
}

impl<T> From<TypedHashPointer<T>> for HashPointer {
    fn from(typed_hash_pointer: TypedHashPointer<T>) -> Self {
        typed_hash_pointer.hash_pointer
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

#[derive(Encode, Decode, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct TypedHashPointer<T> {
    hash_pointer: HashPointer,
    _marker: PhantomData<T>,
}

impl<T> Clone for TypedHashPointer<T> {
    fn clone(&self) -> Self {
        Self {
            hash_pointer: self.hash_pointer.clone(),
            _marker: self._marker,
        }
    }
}

impl<T> TypedHashPointer<T> {
    pub fn new(hash_pointer: HashPointer) -> Self {
        Self {
            hash_pointer,
            _marker: PhantomData,
        }
    }
}

pub trait HashPointerReference {
    fn set(&mut self, value: &HashPointer);

    fn get(&mut self) -> HashPointer;
}

pub trait TypedHashPointerReference<T>: HashPointerReference {
    fn set_typed(&mut self, value: &TypedHashPointer<T>) {
        self.set(&value.into())
    }
    fn get_typed(&mut self) -> TypedHashPointer<T> {
        TypedHashPointer::new(self.get())
    }
}

impl<U, T: HashPointerReference> TypedHashPointerReference<U> for T {}

pub struct InMemoryHashPointerReference {
    value: HashPointer,
}

impl InMemoryHashPointerReference {
    pub fn new(value: HashPointer) -> Self {
        Self { value }
    }
}

impl HashPointerReference for InMemoryHashPointerReference {
    fn set(&mut self, value: &HashPointer) {
        self.value = value.clone();
    }

    fn get(&mut self) -> HashPointer {
        self.value.clone()
    }
}
