use std::collections::BTreeSet;

use crate::hash_pointer::HashPointer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Manifest {
    pub references: BTreeSet<HashPointer>,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            references: BTreeSet::new(),
        }
    }

    pub fn add_reference(&mut self, reference: HashPointer) {
        self.references.insert(reference);
    }

    pub fn remove_reference(&mut self, reference: &HashPointer) {
        self.references.remove(reference);
    }

    pub fn has_reference(&self, reference: &HashPointer) -> bool {
        self.references.contains(reference)
    }
}
