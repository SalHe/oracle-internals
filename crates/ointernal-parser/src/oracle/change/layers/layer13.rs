use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{
        ChangeAndVector,
        cv::{ConvertError, FromCV},
    },
    redo::DBA,
    scn::Scn,
};

#[derive(KnownLayout, Immutable, FromBytes, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ObjInfo {
    pub obj: u32,
    pub type_: u16,
    pub itls: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockFormat<'a> {
    pub obj_info: &'a ObjInfo,
    pub scn: &'a Scn,
}

impl<'a> FromCV<'a> for BlockFormat<'a> {
    fn from_cv(cv: &'a ChangeAndVector) -> Result<Self, ConvertError> {
        Ok(Self {
            obj_info: FromBytes::ref_from_bytes(&cv.vector[0])?,
            scn: FromBytes::ref_from_bytes(&cv.vector[1])?,
        })
    }
}

#[derive(Debug, FromPrimitive)]
pub enum BlockLinkModifyOp {
    /// LCLR (lock clear)
    LockClear = 1,

    /// LSET (lock set)
    LockSet = 2,

    /// LWRT (lock write)
    LockWrite = 3,

    /// LCLK (clear lock)
    ClearLock = 4,
    // 5..: unknown
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct BlockLinkModifyData {
    pub op_code: u8,
    _unknown_1: [u8; 3],
    pub dba: DBA,
    pub itli: u32,
}

impl BlockLinkModifyData {
    /// Get op code if recognized.
    pub fn op_code(&self) -> Option<BlockLinkModifyOp> {
        BlockLinkModifyOp::from_u8(self.op_code)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockLinkModify<'a> {
    pub data: &'a BlockLinkModifyData,
}

impl<'a> FromCV<'a> for BlockLinkModify<'a> {
    fn from_cv(cv: &'a ChangeAndVector) -> Result<Self, ConvertError> {
        Ok(Self {
            data: FromBytes::ref_from_bytes(&cv.vector[0])?,
        })
    }
}
