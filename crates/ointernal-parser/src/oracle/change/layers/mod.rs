use zerocopy::{FromBytes, Immutable, KnownLayout};

pub mod layer13;
pub mod layer5;

#[derive(KnownLayout, Immutable, FromBytes, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Xid {
    /// Undo segment number (USN) of transaction
    pub usn: u16,
    /// Undo segment header transaction table slot
    pub htt_slot: u16,
    /// Sequence number (wrap)
    pub sequence: u32,
}
