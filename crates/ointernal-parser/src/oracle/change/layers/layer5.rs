use bitfield_struct::bitfield;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{ConvertError, cv::FromCV, layers::Xid},
    redo::UBA,
    scn::Scn,
};

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct UndoHeaderRedoData {
    pub slt: u16,
    /// #{AMND}
    _unknown_1: [u8; 2],
    pub sqn: u32,
    pub uba: UBA,
    pub flag: UndoHeaderRedoDataFlag,
    pub size: u16,
    pub fbi: u8,
    _unknown_2: [u8; 3],
    pub parent_xid: Xid,
}

#[bitfield(u16)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct UndoHeaderRedoDataFlag {
    #[bits(6)]
    unknown: u16,
    #[bits(1)]
    pub pdb_uid: bool,
    #[bits(9)]
    unknown2: u16,
}

#[derive(Debug)]
pub struct UndoHeaderRedo<'a> {
    pub data: &'a UndoHeaderRedoData,
    pub pdbuid: Option<&'a u32>,
}

impl<'a> FromCV<'a> for UndoHeaderRedo<'a> {
    fn from_cv(cv: &'a crate::oracle::change::ChangeAndVector) -> Result<Self, ConvertError> {
        let data = FromBytes::ref_from_bytes(&cv.vector[0])?;
        Ok(Self {
            data,
            pdbuid: if data.flag.pdb_uid() {
                Some(FromBytes::ref_from_bytes(&cv.vector[1])?)
            } else {
                None
            },
        })
    }
}

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct ChangeTxSlotStatusData {
    pub slot: u16,
    /// #{AMND}
    _unknown_1: u16,
    pub seq: u16,
    /// #{AMND}
    _unknown_2: [u8; 2],
    pub srt: u32,
    pub sta: i32,

    /// #{AME} AA BB CC DD -> oracledb.exceptions.DatabaseError: DPY-4011: the database or network closed the connection
    /// 0x01: [ChangeTxSlotStatus::scn] is [None] if 0x01 is not set.
    /// 0x02: [ChangeTxSlotStatusData2] is [None] if 0x02 not set.
    pub flags: u8,
    _unknown_3: [u8; 3],
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct ChangeTxSlotStatusData2 {
    pub uba: UBA,
    pub ext: u16,
    pub spc: u16,
    pub fbi: u8,
    _unknown_1: [u8; 3],
}

#[derive(Debug)]
pub struct ChangeTxSlotStatus<'a> {
    pub data: &'a ChangeTxSlotStatusData,
    pub data2: Option<&'a ChangeTxSlotStatusData2>,
    pub scn: Option<&'a Scn>,
}

impl<'a> FromCV<'a> for ChangeTxSlotStatus<'a> {
    fn from_cv(cv: &'a crate::oracle::change::ChangeAndVector) -> Result<Self, ConvertError> {
        let data = FromBytes::ref_from_bytes(&cv.vector[0])?;
        // cv.data[2] ???
        Ok(Self {
            data,
            data2: if data.flags & 0x02 != 0 {
                Some(FromBytes::ref_from_bytes(&cv.vector[1])?)
            } else {
                None
            },
            scn: if data.flags & 0x01 != 0 {
                // TODO from ???
                Some(FromBytes::ref_from_prefix(&cv.vector[2])?.0)
            } else {
                None
            },
        })
    }
}

// -----------------------------------------------------------------------------
