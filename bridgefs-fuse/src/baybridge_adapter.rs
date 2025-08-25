use baybridge::{
    client::Actions,
    models::{ContentBlock, Name, Value},
};
use bridgefs_core::{
    content_store::ContentStore,
    hash_pointer::{HashPointer, HashPointerReference, TypedHashPointer},
    index::Index,
};

pub struct BaybridgeAdapter {
    runtime: tokio::runtime::Runtime,
    actions: Actions,
}

impl BaybridgeAdapter {
    pub fn new(actions: Actions) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        Self { runtime, actions }
    }

    pub fn content_store(&self) -> BaybridgeContentStore<'_> {
        BaybridgeContentStore { adapter: self }
    }

    pub fn hash_pointer_reference(
        &self,
        default_value: TypedHashPointer<Index>,
    ) -> BaybridgeHashPointerReference<'_> {
        BaybridgeHashPointerReference {
            name: Name::new("filesystem2".to_string()),
            default_value,
            adapter: self,
        }
    }
}

pub struct BaybridgeContentStore<'a> {
    adapter: &'a BaybridgeAdapter,
}

pub struct BaybridgeHashPointerReference<'a> {
    name: Name,
    default_value: TypedHashPointer<Index>,
    adapter: &'a BaybridgeAdapter,
}

impl ContentStore for BaybridgeContentStore<'_> {
    fn add_content(&mut self, content: &[u8]) -> HashPointer {
        let content_block = ContentBlock {
            data: content.to_vec(),
            references: Vec::new(),
        };
        self.adapter
            .runtime
            .block_on(self.adapter.actions.set_immutable(content_block))
            .unwrap()
            .into()
    }

    fn get_content(&self, hash: &HashPointer) -> Vec<u8> {
        let content_block = self
            .adapter
            .runtime
            .block_on(self.adapter.actions.get_immutable(&hash.into()))
            .unwrap();
        content_block.data
    }
}

impl HashPointerReference for BaybridgeHashPointerReference<'_> {
    fn set(&mut self, value: &HashPointer) {
        let serialized_value = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
        let value = Value::new(serialized_value);

        // TODO: find a way to get an increasing priority, probably need baybridge to support strong reads
        self.adapter
            .runtime
            .block_on(
                self.adapter
                    .actions
                    .set()
                    .name(self.name.clone())
                    .value(value)
                    .call(),
            )
            .unwrap()
    }

    fn get(&mut self) -> HashPointer {
        match self.get_internal() {
            Some(hash_pointer) => hash_pointer,
            None => {
                let default_value = (&self.default_value).into();
                self.set(&default_value);
                default_value
            }
        }
    }
}

impl BaybridgeHashPointerReference<'_> {
    fn get_internal(&self) -> Option<HashPointer> {
        let value = self
            .adapter
            .runtime
            .block_on(self.adapter.actions.get_mine(&self.name))
            .ok()?;
        Some(
            bincode::decode_from_slice(value.as_bytes(), bincode::config::standard())
                .unwrap()
                .0,
        )
    }
}
