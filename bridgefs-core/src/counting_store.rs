use bincode::{Decode, Encode};

use crate::{
    content_store::{ContentStore, ParsingContentStoreExt},
    hash_pointer::TypedHashPointer,
    manifest::Manifest,
};

#[derive(Debug)]
pub struct CountingStore<T: ContentStore> {
    store: T,
    manifest: Manifest,
}

pub trait HasReferences<StoreT: ContentStore> {
    fn delete_references(&self, new_value: Option<&Self>, store: &mut CountingStore<StoreT>);
}

impl<StoreT: ContentStore> CountingStore<StoreT> {
    pub fn new(store: StoreT, manifest: Manifest) -> CountingStore<StoreT> {
        CountingStore { store, manifest }
    }

    pub fn get_parsed<U: Decode<()>>(&self, hash: &TypedHashPointer<U>) -> U {
        self.store.get_parsed(hash)
    }

    pub fn store_new_content<T: Encode>(&mut self, value: &T) -> TypedHashPointer<T> {
        let hash = self.store.add_parsed(value);
        self.manifest.add_reference((&hash).into());
        hash
    }

    pub fn delete_content<T: Encode + Decode<()> + HasReferences<StoreT>>(
        &mut self,
        hash: &TypedHashPointer<T>,
    ) {
        let item_to_delete: T = self.get_parsed(hash);
        self.manifest.remove_reference(hash.into());
        item_to_delete.delete_references(None, self);
    }

    pub fn replace_content<T: Encode + Decode<()> + HasReferences<StoreT>>(
        &mut self,
        previous: &TypedHashPointer<T>,
        value: &T,
    ) -> TypedHashPointer<T> {
        let item_to_delete: T = self.get_parsed(previous);
        self.manifest.remove_reference(previous.into());
        item_to_delete.delete_references(Some(value), self);
        self.store_new_content(value)
    }
}
