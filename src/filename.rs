use std::os::unix::ffi::OsStringExt;

use bincode::{Decode, Encode};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Filename {
    pub name: Vec<u8>,
}

impl From<&std::ffi::OsStr> for Filename {
    fn from(os_str: &std::ffi::OsStr) -> Self {
        Filename {
            name: os_str.as_encoded_bytes().to_vec(),
        }
    }
}

impl Into<std::ffi::OsString> for Filename {
    fn into(self) -> std::ffi::OsString {
        std::ffi::OsString::from_vec(self.name)
    }
}
