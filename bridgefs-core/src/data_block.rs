use bincode::{Decode, Encode};

use crate::{content_store::ContentStore, counting_store::HasReferences};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, Default)]
pub struct DataBlock {
    pub data: Vec<u8>,
}

impl DataBlock {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<StoreT: ContentStore> HasReferences<StoreT> for DataBlock {
    fn delete_references(&self, _store: &mut crate::counting_store::CountingStore<StoreT>) {
        // no-op
    }
}
