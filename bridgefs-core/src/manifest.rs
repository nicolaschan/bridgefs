use std::collections::BTreeSet;

use crate::hash_pointer::HashPointer;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Manifest {
    pub references: BTreeSet<HashPointer>,
}
