use bincode::Decode;

use crate::hash_pointer::{HashPointer, TypedHashPointer};

pub trait ContentStore {
    fn add_content(&mut self, content: &[u8]) -> HashPointer;

    fn get_content(&self, hash: &HashPointer) -> Vec<u8>;
}

#[derive(Default, Debug)]
pub struct InMemoryContentStore {
    store: std::collections::HashMap<HashPointer, Vec<u8>>,
}

impl ContentStore for InMemoryContentStore {
    fn add_content(&mut self, content: &[u8]) -> HashPointer {
        let hash: HashPointer = blake3::hash(content).into();
        self.store.insert(hash.clone(), content.to_vec());
        hash
    }

    fn get_content(&self, hash: &HashPointer) -> Vec<u8> {
        self.store.get(hash).cloned().unwrap_or_default()
    }
}

pub trait ParsingContentStoreExt: ContentStore {
    fn get_parsed<U: Decode<()>>(&self, hash: &TypedHashPointer<U>) -> U {
        let bytes = self.get_content(&hash.into());
        bincode::decode_from_slice::<U, _>(&bytes, bincode::config::standard())
            .unwrap()
            .0
    }

    fn add_parsed<U: bincode::Encode>(&mut self, value: &U) -> TypedHashPointer<U> {
        let bytes = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
        let hash_pointer = self.add_content(&bytes);
        TypedHashPointer::new(hash_pointer)
    }
}

impl<T: ContentStore> ParsingContentStoreExt for T {}
