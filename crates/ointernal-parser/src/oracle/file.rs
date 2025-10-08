use zerocopy::{FromBytes, Immutable, KnownLayout};

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FileHeader {
    _unknown_1: [u8; 20],
    pub block_size: u32,
    pub block_count: u32,
    pub magic_number: u32,
    _unknown_2: u32,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FileHeaderBlock {
    pub header: FileHeader,
    pub body: [u8],
}
