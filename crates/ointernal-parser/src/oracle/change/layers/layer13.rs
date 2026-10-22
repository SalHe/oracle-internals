use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{
        VectorData,
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
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            obj_info: FromBytes::ref_from_bytes(&v[0].data)?,
            scn: FromBytes::ref_from_bytes(&v[1].data)?,
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
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            data: FromBytes::ref_from_bytes(&v[0].data)?,
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
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let mut modifies = Vec::new();
        for v in v {
            modifies.push(FreelistModifyDataHead::ref_from_bytes(&v.data)?);
        }
        Ok(Self { modifies })
    }
}

// -----------------------------------------------------------------------------

/// `KTSL - Format LOB Hash Bucket Block` (OP:13.43), 48 bytes.
///
/// `flag` bits 0 and 1 select the bucket kind: bit 0 set means committed free
/// space (CFS), otherwise bit 1 set means uncommitted free space (UFS), and any
/// other value is `(Invalid)`. For CFS, `range` selects the size class printed
/// after the flag (it is ignored for the other kinds).
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FormatLobHashBucketBlockHead {
    /// Size class: 0 = 2k-32k, 1 = 32k-64k, ... 6 = 1m-64m, else invalid.
    pub range: u32,
    pub inst: u32,
    /// `#{AMND}`: non-zero values produced no output change.
    _unknown_1: [u32; 3],
    /// Bit 0: CFS when set. Otherwise bit 1: UFS when set. Neither: invalid.
    pub flag: u32,
    /// `#{AMND}`: non-zero values produced no output change.
    _unknown_2: [u32; 4],
    pub objd: u32,
    /// `#{AMND}`: 0 in every observed record.
    pub inc: u32,
}

#[derive(Debug)]
pub struct FormatLobHashBucketBlock<'a> {
    pub head: &'a FormatLobHashBucketBlockHead,
}

impl<'a> FromCV<'a> for FormatLobHashBucketBlock<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            head: FromBytes::ref_from_bytes(&v[0].data[..v[0].size as usize])?,
        })
    }
}

// -----------------------------------------------------------------------------

/// `ktspbfredo - Format Pagetable Datablock` (OP:13.21), first vector, 20 bytes.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FormatPagetableDatablockHead {
    pub objd: u32,
    /// Bit 15 is not part of the printed type; it selects whether Oracle prints
    /// the `cscn` line. The printed value is `typ & 0x7fff`.
    pub typ: u16,
    /// Printed sign-extended to 64 bits (e.g. `0xff02` becomes
    /// `18446744073709551362`).
    pub itls: i16,
    pub fmt_flag: u32,
    /// Parent level-1 bitmap block DBA.
    pub parent_dba: u32,
    pub poff: u8,
    /// `#{AMND}`: non-zero values produced no output change.
    _unknown: [u8; 3],
}

/// `ktspbfredo - Format Pagetable Datablock` (OP:13.21), second vector, 16 bytes.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FormatPagetableDatablockTail {
    /// Creation SCN; the full 64-bit value is printed.
    pub cscn: Scn,
    pub inc: u32,
    /// `#{AMND}`: non-zero values produced no output change.
    _unknown: u32,
}

#[derive(Debug)]
pub struct FormatPagetableDatablock<'a> {
    pub head: &'a FormatPagetableDatablockHead,
    pub tail: &'a FormatPagetableDatablockTail,
}

impl<'a> FromCV<'a> for FormatPagetableDatablock<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            head: FromBytes::ref_from_bytes(&v[0].data[..v[0].size as usize])?,
            tail: FromBytes::ref_from_bytes(&v[1].data[..v[1].size as usize])?,
        })
    }
}

// -----------------------------------------------------------------------------
