use bincode::Decode;

use crate::hash_pointer::HashPointer;

pub trait ContentStore {
    fn add_content(&mut self, content: &[u8]) -> HashPointer;

    fn get_content(&mut self, hash: &HashPointer) -> Vec<u8>;

    fn remove_content(&mut self, hash: &HashPointer);
}

#[derive(Debug)]
pub struct InMemoryContentStore {
    store: std::collections::HashMap<HashPointer, Vec<u8>>,
}

impl InMemoryContentStore {
    pub fn new() -> Self {
        InMemoryContentStore {
            store: std::collections::HashMap::new(),
        }
    }
}

impl ContentStore for InMemoryContentStore {
    fn add_content(&mut self, content: &[u8]) -> HashPointer {
        let hash: HashPointer = blake3::hash(content).into();
        self.store.insert(hash.clone(), content.to_vec());
        hash
    }

    fn get_content(&mut self, hash: &HashPointer) -> Vec<u8> {
        self.store.get(hash).cloned().unwrap_or_default()
    }

    fn remove_content(&mut self, hash: &HashPointer) {
        self.store.remove(&hash);
    }
}

#[derive(Debug)]
pub struct ParsingContentStore<T: ContentStore> {
    inner: T,
}

impl<T: ContentStore> ParsingContentStore<T> {
    pub fn new(inner: T) -> Self {
        ParsingContentStore { inner }
    }

    pub fn get_parsed<U: Decode<()>>(&mut self, hash: &HashPointer) -> U {
        let bytes = self.inner.get_content(hash);
        bincode::decode_from_slice::<U, _>(&bytes, bincode::config::standard())
            .unwrap()
            .0
    }

    pub fn add_parsed<U: bincode::Encode>(&mut self, value: &U) -> HashPointer {
        let bytes = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
        self.inner.add_content(&bytes)
    }
}

impl<T: ContentStore> ContentStore for ParsingContentStore<T> {
    fn add_content(&mut self, content: &[u8]) -> HashPointer {
        self.inner.add_content(content)
    }

    fn get_content(&mut self, hash: &HashPointer) -> Vec<u8> {
        self.inner.get_content(hash)
    }

    fn remove_content(&mut self, hash: &HashPointer) {
        self.inner.remove_content(hash);
    }
}
