use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{
        ChangeAndVector,
        cv::{ConvertError, FromCV},
        layers::Xid,
    },
    redo::DBA,
    scn::Scn,
};

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct ObjInfo {
    pub obj: u32,
    pub type_: u16,
    pub itls: u16,
}

#[derive(Debug)]
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

// -----------------------------------------------------------------------------

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

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
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

#[derive(Debug)]
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

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FreelistModifyDataHead {
    pub op: u8,
    _unknown_1: [u8; 3],
    body: [u8],
}

impl FreelistModifyDataHead {
    /// Get op and data.
    ///
    /// If [Self::op] is not recognized, return Ok(None).
    /// If [Self::op] is recognized, return Some(op, body).
    pub fn data(
        &'_ self,
    ) -> Result<Option<(FreelistModifyBaseOp, FreelistModifyDataBody<'_>)>, ConvertError> {
        let op = FromPrimitive::from_u8(self.op);
        match op {
            Some(op) => match op {
                FreelistModifyBaseOp::MoveHwm => Ok(Some((
                    op,
                    FreelistModifyDataBody::MoveHwm {
                        nbk: *(FromBytes::ref_from_bytes(&self.body)?),
                    },
                ))),
                _ => Ok(Some((
                    op,
                    FreelistModifyDataBody::Other(FromBytes::ref_from_bytes(&self.body)?),
                ))),
            },
            None => Ok(None),
        }
    }
}

#[derive(FromPrimitive, Debug)]
#[repr(C)]
pub enum FreelistModifyBaseOp {
    /// 1. HWMMV (move hwm)
    MoveHwm = 1,

    /// 2. LNEW (list new)
    ListNew = 2,

    /// 3. LUPD_LBLK (link block)
    LinkBlock = 3,

    /// 4. LUPD_UNLBLK (unlink block)
    UnlinkBlock = 4,

    /// 5. LUPD_LLIST (link a list)
    LinkList = 5,

    /// 6. LUPD_UNLIST (unlink a list)
    UnlinkList = 6,

    /// 7. GSET_SEGLIST (seg lists copy)
    SegListsCopy = 7,

    /// 8. GSET_XNTLIST (xnt lists copy)
    XNTListsCopy = 8,
    // 9. unknown...
}

pub enum FreelistModifyDataBody<'a> {
    MoveHwm { nbk: u32 },
    Other(&'a FreelistModifyDataBodyOther),
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FreelistModifyDataBodyOther {
    pub slot_count: u32,
    pub slot_no: u32,
    pub flag: u32,
    pub xid: Xid,
    pub head: u32,
    pub tail: u32,
}

#[derive(Debug)]
pub struct FreelistModify<'a> {
    pub modifies: Vec<&'a FreelistModifyDataHead>,
}

impl<'a> FromCV<'a> for FreelistModify<'a> {
    fn from_cv(cv: &'a ChangeAndVector) -> Result<Self, ConvertError> {
        let mut modifies = Vec::new();
        for v in &cv.vector {
            modifies.push(FreelistModifyDataHead::ref_from_bytes(v)?);
        }
        Ok(Self { modifies })
    }
}

// -----------------------------------------------------------------------------
