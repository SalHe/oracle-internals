use bitfield_struct::bitfield;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{
        ConvertError, VectorData,
        cv::FromCV,
        layers::{Xid, layer4::BlockCleanOutRecord},
    },
    redo::{DBA, UBA},
};

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RowOperation {
    pub b1: IRPByte1,
    pub b2: IRPByte2,
    padding: [u8; 2],
    rest: [u8],
}

impl RowOperation {
    pub fn rest(&'_ self) -> Result<Option<InsertRowPieceOpRest<'_>>, ConvertError> {
        let rest = if self.b2.start_from_8() {
            &self.rest[4..]
        } else {
            &self.rest
        };

        self.b1.clean_out();

        let r = match self.b1.op() {
            1 => {
                let (f, rest) = InsertRowPieceOpRestF::ref_from_prefix(rest)?;
                let block_clean = if self.b1.clean_out() {
                    Some(BlockCleanOutRecord::parse(rest)?)
                } else {
                    None
                };

                Some(InsertRowPieceOpRest::F { f, block_clean })
            }
            2 => Some(InsertRowPieceOpRest::C(FromBytes::ref_from_bytes(rest)?)),
            3 => Some(InsertRowPieceOpRest::Z),
            4 => Some(InsertRowPieceOpRest::L),
            5 => Some(InsertRowPieceOpRest::R),
            6 => Some(InsertRowPieceOpRest::N),
            8 => Some(InsertRowPieceOpRest::G),
            _ => None,
        };

        Ok(r)
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct InsertRowPieceOpRestF {
    pub xid: Xid,
    pub uba: UBA,
}

pub enum InsertRowPieceOpRest<'a> {
    F {
        f: &'a InsertRowPieceOpRestF,
        block_clean: Option<BlockCleanOutRecord<'a>>,
    },
    C(&'a UBA),
    Z,
    L,
    R,
    N,
    G,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct IRPByte1 {
    /// 1. F
    /// 2. C
    /// 3. Z
    /// 4. L
    /// 5. R
    /// 6. N
    /// 8. G
    /// otherwise, invalid op
    #[bits(4)]
    pub op: u8,

    /// Block cleanout record, scn:  0x0000000000000000 ver: 0x00 opt: 0x00 bigscn: N compact: N spare: 00000000, entries follow...
    #[bits(1)]
    pub clean_out: bool,

    #[bits(3)]
    pub unknown: u8,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct IRPByte2 {
    #[bits(2)]
    pub version: u8,

    /// 0. pre-11
    /// 1. post-11
    #[bits(1)]
    pub post_11: bool,

    #[bits(1)]
    pub start_from_8: bool,

    /// #{AMND}: [IRPByte2] is 0x04 and [IRPByte2] is 0x14.
    /// #{AMND}: [IRPByte2] is 0x00 and [IRPByte2] is 0x10.
    /// #{AMND}: [IRPByte2] is 0x0D/0x2D/0x4D/0x8D.
    #[bits(4)]
    unknown: u8,
}

/// KDO Op code: ....
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMeta {
    pub bdba: DBA,
    pub hdba: DBA,

    pub maxfr: u16,
    pub op: IRPMetaByte1,
    pub xtype: IRPMetaByte2,

    pub itli: u8,
    pub ispac: u8,
    _unknown_1: [u8; 2],

    rest: [u8],
}

impl RPMeta {
    pub(crate) fn rest<'a>(
        &'a self,
        v: &'a [VectorData],
    ) -> Result<Option<RPMetaRest<'a>>, ConvertError> {
        let op = KdoOp::from_u8(self.op.op());
        if let Some(op) = op {
            let rest = match op {
                KdoOp::IUR => RPMetaRest::IUR,
                KdoOp::IRP => RPMetaRest::IRP(FromBytes::ref_from_bytes(&self.rest)?),
                KdoOp::DRP => RPMetaRest::DRP(FromBytes::ref_from_bytes(&self.rest)?),
                KdoOp::LKR => RPMetaRest::LKR,
                KdoOp::URP => RPMetaRest::URP,
                KdoOp::ORP => RPMetaRest::ORP,
                KdoOp::MFC => RPMetaRest::MFC,
                KdoOp::CFA => RPMetaRest::CFA,
                KdoOp::CKI => RPMetaRest::CKI,
                KdoOp::SKL => RPMetaRest::SKL,
                KdoOp::QMI => RPMetaRest::QMI,
                KdoOp::QMD => RPMetaRest::QMD,
                KdoOp::ArrayUpdate => {
                    const SINGLE_SIZE: usize = 28; // TODO how to determine size of single entry?
                    let row_count = *u32::ref_from_bytes(&self.rest)?;
                    let rows = v[0]
                        .data
                        .chunks_exact(SINGLE_SIZE)
                        .map(|x| -> Result<RPMetaWrapper<'_>, ConvertError> {
                            Ok(RPMetaWrapper {
                                meta: RPMeta::ref_from_bytes(x)?,
                                v: &v[1..], // columns' data is saved in single vector, and shared across different UPDATEs.
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    RPMetaRest::ArrayUpdate(RPMeta19 { row_count, rows })
                }
                KdoOp::DSC => RPMetaRest::DSC,
                KdoOp::ArrayUpdateContinue => RPMetaRest::ArrayUpdateContinue,
                KdoOp::LMN => RPMetaRest::LMN,
                KdoOp::LLB => RPMetaRest::LLB,
                KdoOp::SHK => RPMetaRest::SHK,
                KdoOp::CMP => RPMetaRest::CMP,
                KdoOp::DCU => RPMetaRest::DCU,
                KdoOp::MRK => RPMetaRest::MRK,
                KdoOp::IRP2 => RPMetaRest::IRP2,
                KdoOp::DRP2 => RPMetaRest::DRP2,
                KdoOp::CMP2 => RPMetaRest::CMP2,
                KdoOp::DGD => RPMetaRest::DGD,
            };

            Ok(Some(rest))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct RPMetaWrapper<'a> {
    pub meta: &'a RPMeta,
    v: &'a [VectorData],
}

impl RPMetaWrapper<'_> {
    pub fn rest(&self) -> Result<Option<RPMetaRest<'_>>, ConvertError> {
        self.meta.rest(self.v)
    }

    /// `VARCHAR(N)`: raw string bytes. 'AAAA' -> 41414141 (hex)
    /// `NUMBER`: [crate::oracle::types::get_number_from_bytes]
    pub fn col_data(&self, meta: &RPMetaIRP, col: usize) -> Option<Option<&'_ [u8]>> {
        meta.is_col_null(col as _).map(|is_null| {
            if is_null {
                None
            } else {
                let i = col;
                Some(&self.v[i].data[..(self.v[i].size as _)])
            }
        })
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMetaIRP {
    pub fb_flags: IRPFbFlags,
    pub lb: u8,
    pub column_count: u8,

    /// Used when [IRPFbFlags::c] of [Self::fb_flags] is true.
    pub cki: u8,

    _unknown_3: [u8; 8],

    pub nrid_dba: DBA,
    pub nrid_slot: u16,
    _unknown_4: [u8; 6],

    pub delta_size: u16,
    pub slot: u16,
    pub tabn: i8,
    nulls: [u8],
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMetaDRP {
    pub slot: u16,
    pub tabn: i8,
    nulls: [u8],
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMeta19<'a> {
    pub row_count: u32,
    pub rows: Vec<RPMetaWrapper<'a>>,
}

pub enum RPMetaRest<'a> {
    IUR,
    IRP(&'a RPMetaIRP),
    DRP(&'a RPMetaDRP),
    LKR,
    URP,
    ORP,
    MFC,
    CFA,
    CKI,
    SKL,
    QMI,
    QMD,
    ArrayUpdate(RPMeta19<'a>),
    DSC,
    ArrayUpdateContinue,
    LMN,
    LLB,
    SHK,
    CMP,
    DCU,
    MRK,
    IRP2,
    DRP2,
    CMP2,
    DGD,
}

impl RPMetaIRP {
    pub fn is_col_null(&self, col: u8) -> Option<bool> {
        if col < self.column_count {
            Some((self.nulls[(col as usize) / 8] >> (col % 8)) & 1 != 0)
        } else {
            None
        }
    }
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct IRPFbFlags {
    /// The last column continues in the Next piece
    #[bits(1)]
    pub n: bool,

    /// The first column continues from the Previous piece
    #[bits(1)]
    pub p: bool,

    /// Last ctx piece
    #[bits(1)]
    pub l: bool,

    /// First ctx piece
    #[bits(1)]
    pub f: bool,

    /// Deleted row
    #[bits(1)]
    pub d: bool,

    /// Head piece of row
    #[bits(1)]
    pub h: bool,

    /// Clustered table member
    #[bits(1)]
    pub c: bool,

    /// Cluster Key
    #[bits(1)]
    pub k: bool,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct IRPMetaFlags {
    fwd: bool,
    bkw: bool,
    #[bits(6)]
    unknown: u8,
}

#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum KdoOp {
    IUR = 0x01,
    IRP = 0x02,
    DRP = 0x03,
    LKR = 0x04,
    URP = 0x05,
    ORP = 0x06,
    MFC = 0x07,
    CFA = 0x08,
    CKI = 0x09,
    SKL = 0x0A,
    QMI = 0x0B,
    QMD = 0x0C,
    DSC = 0x0E,
    LMN = 0x10,
    LLB = 0x11,

    ArrayUpdate = 0x13,

    SHK = 0x14,

    ArrayUpdateContinue = 0x15,
    CMP = 0x16,
    DCU = 0x17,
    MRK = 0x18,
    IRP2 = 0x1A,
    DRP2 = 0x1B,
    CMP2 = 0x1C,
    DGD = 0x1D,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct IRPMetaByte1 {
    /// [KdoOp]
    #[bits(5)]
    pub op: u8,

    #[bits(1)]
    unknown_1: u8,

    #[bits(1)]
    pub row_dependency: bool,

    #[bits(1)]
    unknown_2: u8,
}

#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum IRPXtype {
    /// Redo
    XA = 1,
    /// Rollback
    XR = 2,
    CR = 3,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct IRPMetaByte2 {
    /// [IRPXtype]
    #[bits(2)]
    pub typ: u8,

    #[bits(5)]
    flags: u8,

    #[bits(1)]
    pub kdo_kdom2: bool,
}

#[derive(Debug)]
pub struct KtbRedo<'a> {
    pub op: &'a RowOperation,
    pub meta: RPMetaWrapper<'a>,
}

impl<'a> FromCV<'a> for KtbRedo<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            op: FromBytes::ref_from_bytes(&v[0].data)?,
            meta: RPMetaWrapper {
                meta: FromBytes::ref_from_bytes(&v[1].data)?,
                v: &v[2..],
            },
        })
    }
}

// -----------------------------------------------------------------------------

#[cfg(test)]
mod test {
    // use std::mem::offset_of;

    // use crate::oracle::change::layers::layer11::InsertRowPieceMeta;

    #[test]
    fn test_offset() {
        // assert!(
        //     offset_of!(InsertRowPieceMeta, bkw_low) - offset_of!(InsertRowPieceMeta, fwd_pos) == 2
        // );
        // assert!(offset_of!(InsertRowPieceMeta, flags) % 4 == 0);
    }
}
