use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{ConvertError, cv::FromCV, layers::Xid},
    redo::UBA,
};

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct UndoHeaderRedoData {
    pub slt: u16,
    /// #{AMND}
    _unknown_1: [u8; 2],
    pub sqn: u32,
    pub uba: UBA,
    pub flag: u16,
    pub size: u16,
    pub fbi: u8,
    _unknown_2: [u8; 3],
    pub parent_xid: Xid,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UndoHeaderRedo<'a> {
    pub data: &'a UndoHeaderRedoData,
    pub pdbuid: &'a u32,
}

impl<'a> FromCV<'a> for UndoHeaderRedo<'a> {
    fn from_cv(cv: &'a crate::oracle::change::ChangeAndVector) -> Result<Self, ConvertError> {
        Ok(Self {
            data: FromBytes::ref_from_bytes(&cv.vector[0])?,
            pdbuid: FromBytes::ref_from_bytes(&cv.vector[1])?,
        })
    }
}

// -----------------------------------------------------------------------------
