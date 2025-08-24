use bincode::{Decode, Encode};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, Default)]
pub struct DataBlock {
    pub data: Vec<u8>,
}
