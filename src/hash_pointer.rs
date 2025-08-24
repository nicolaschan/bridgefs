use std::fmt::Debug;

use bincode::{Decode, Encode};

#[derive(Encode, Decode, Hash, PartialEq, Eq, Clone)]
pub struct HashPointer {
    bytes: [u8; 32],
}

impl From<blake3::Hash> for HashPointer {
    fn from(blake3_hash: blake3::Hash) -> Self {
        HashPointer {
            bytes: blake3_hash.as_bytes().clone(),
        }
    }
}

impl Debug for HashPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HashPointer({})", base64::encode(self.bytes))
    }
}
