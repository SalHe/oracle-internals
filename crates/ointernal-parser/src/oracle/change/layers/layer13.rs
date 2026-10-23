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

/// Sub-operation of `UpdateLhwmHhwm` (OP:13.28), first dword of the vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateLhwmHhwmOp {
    /// 1: `Lasts in Header`
    LastsInHeader,
    /// 2: `High HWM`
    HighHwm,
    /// 3: `Low HWM`
    LowHwm,
    /// 4: `Both the HWMs`
    BothHwm,
    /// 6: `hint dba:`
    HintDba,
    /// 9: `Update Express Allocation Area`
    ExpressAllocationArea,
    /// 0, 5, 7, 8 and anything else: Oracle prints nothing at all.
    Unknown(u32),
}

impl UpdateLhwmHhwmOp {
    fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::LastsInHeader,
            2 => Self::HighHwm,
            3 => Self::LowHwm,
            4 => Self::BothHwm,
            6 => Self::HintDba,
            9 => Self::ExpressAllocationArea,
            other => Self::Unknown(other),
        }
    }
}

/// One high-water-mark descriptor (32 bytes).
///
/// The field order is the recorded one and differs from the printed order.
/// `OP:13.22` carries the same descriptor after a 4-byte header.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct Hwm {
    /// `ext#`
    pub ext: u32,
    /// `blk#`
    pub blk: u32,
    /// `ext size`
    pub ext_size: u32,
    /// `Highwater` DBA
    pub dba: u32,
    /// `mapblk`
    pub mapblk: u32,
    /// `offset`
    pub offset: u32,
    /// `#blocks in seg. hdr's freelists`
    pub freelists: u32,
    /// `#blocks below`
    pub below: u32,
}

/// The 80-byte payload of `OP:13.28`.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct UpdateLhwmHhwmData {
    /// [`UpdateLhwmHhwmOp`] as a raw dword.
    pub op: u32,
    /// Descriptor printed by the `High HWM` section.
    pub high: Hwm,
    /// Descriptor printed by the `Low HWM` section.
    pub low: Hwm,
    /// `lfdba:` of the `Low HWM` section.
    pub low_lfdba: u32,
    /// `lfdba:` of the `High HWM` section.
    pub high_lfdba: u32,
    /// `#{AMND}`: it equals `high_lfdba + 1` in every sample and patching it
    /// produced no output change.
    _unknown: u32,
}

#[derive(Debug)]
pub struct UpdateLhwmHhwm<'a> {
    data: &'a UpdateLhwmHhwmData,
}

impl UpdateLhwmHhwm<'_> {
    pub fn data(&self) -> &UpdateLhwmHhwmData {
        self.data
    }

    pub fn op(&self) -> UpdateLhwmHhwmOp {
        UpdateLhwmHhwmOp::from_raw(self.data.op)
    }

    /// `Allocation Area` state of the `Update Express Allocation Area`
    /// sub-operation. Oracle only looks at the low byte of the first
    /// descriptor word: zero prints `EMPTY`, anything else prints `FULL`.
    pub fn express_allocation_full(&self) -> bool {
        self.data.high.ext as u8 != 0
    }
}

impl<'a> FromCV<'a> for UpdateLhwmHhwm<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let len = size_of::<UpdateLhwmHhwmData>();
        if v[0].size < len as u64 {
            return Err(ConvertError::Validity {
                reason: format!(
                    "ktspfhw redo payload is too short for two HWMs: {} bytes",
                    v[0].size
                ),
            });
        }
        Ok(Self {
            data: FromBytes::ref_from_bytes(&v[0].data[..len])?,
        })
    }
}

// -----------------------------------------------------------------------------
