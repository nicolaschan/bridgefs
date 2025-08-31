use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use crate::hash_pointer::HashPointer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Manifest {
    // How many references to each hash pointer
    references: BTreeMap<HashPointer, usize>,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            references: BTreeMap::new(),
        }
    }

    pub fn references(&self) -> BTreeSet<&HashPointer> {
        self.references.keys().collect()
    }

    pub fn add_reference(&mut self, reference: HashPointer) {
        self.references
            .entry(reference)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    pub fn remove_reference(&mut self, reference: HashPointer) {
        let Entry::Occupied(mut e) = self.references.entry(reference) else {
            return;
        };
        *e.get_mut() -= 1;
        if *e.get() == 0 {
            e.remove();
        }
    }

    pub fn has_reference(&self, reference: &HashPointer) -> bool {
        self.references.contains_key(reference)
    }
}
