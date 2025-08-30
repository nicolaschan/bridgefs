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

impl From<Filename> for std::ffi::OsString {
    fn from(filename: Filename) -> Self {
        std::ffi::OsString::from_vec(filename.name)
    }
}

impl From<&str> for Filename {
    fn from(s: &str) -> Self {
        Filename {
            name: s.as_bytes().to_vec(),
        }
    }
}
