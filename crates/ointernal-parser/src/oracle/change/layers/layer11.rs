use std::fmt::{Debug, Display};

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
        wrapper: &'a RPMetaWrapper<'a>,
    ) -> Result<Option<RPMetaRest<'a>>, ConvertError> {
        let op = KdoOp::from_u8(self.op.op());
        if let Some(op) = op {
            let rest = match op {
                KdoOp::IUR => RPMetaRest::IUR,
                KdoOp::IRP => RPMetaRest::IRP(RPMetaIRPWrapper {
                    irp: FromBytes::ref_from_bytes(&self.rest)?,
                    v,
                }),
                KdoOp::DRP => RPMetaRest::DRP(FromBytes::ref_from_bytes(&self.rest)?),
                KdoOp::LKR => RPMetaRest::LKR(FromBytes::ref_from_bytes(&self.rest)?),
                KdoOp::URP => {
                    // TODO what's `rest` for ?
                    let (m, _rest) = FromBytes::ref_from_prefix(&self.rest)?;
                    RPMetaRest::URP(RPMetaURPWrapper {
                        meta: m,
                        cols: <[u16]>::ref_from_bytes(&v[0].data[0..v[0].size as usize])?,
                        data: v[1..]
                            .iter()
                            .map(|x| &x.data[..(x.size as _)])
                            .collect::<Vec<_>>(),
                    })
                }
                KdoOp::ORP => RPMetaRest::ORP(RPMetaIRPWrapper {
                    irp: FromBytes::ref_from_bytes(&self.rest)?,
                    v,
                }),
                KdoOp::MFC => RPMetaRest::MFC,
                KdoOp::CFA => RPMetaRest::CFA,
                KdoOp::CKI => RPMetaRest::CKI,
                KdoOp::SKL => RPMetaRest::SKL,
                KdoOp::QMI => {
                    // TODO what about v[0]?
                    let data: &RPMetaQMI =
                        FromBytes::ref_from_bytes(&v[1].data[0..v[1].size as usize])?;

                    let mut cols = Vec::new();
                    let mut offset = 0usize;
                    while offset < data.col_data.len() {
                        let size = data.col_data[offset] as usize;
                        if size == 0xFF {
                            cols.push(None);
                        } else {
                            cols.push(Some(&data.col_data[offset + 1..offset + 1 + size]));
                            offset += size;
                        }
                        offset += 1;
                    }

                    RPMetaRest::QMI(RPMetaQMIWrapper {
                        meta: FromBytes::ref_from_bytes(&self.rest)?,
                        data,
                        cols,
                        tl: offset + 3, // meta(3 bytes)
                    })
                }
                KdoOp::QMD => RPMetaRest::QMD(FromBytes::ref_from_bytes(&self.rest)?),
                KdoOp::ArrayUpdate => {
                    const SINGLE_SIZE: usize = 28; // TODO how to determine size of single entry?
                    let row_count = self.rest[0];
                    let rows = v[0]
                        .data
                        .chunks_exact(SINGLE_SIZE)
                        .enumerate()
                        .map(|(i_row, x)| -> Result<RPMetaWrapper<'_>, ConvertError> {
                            Ok(RPMetaWrapper {
                                meta: RPMeta::ref_from_bytes(x)?,
                                v: &v[1..], // columns' data is saved in single vector, and shared across different UPDATEs.
                                from_rows: Some(i_row as _),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    RPMetaRest::ArrayUpdate(RPMeta19 { row_count, rows })
                }
                KdoOp::DSC => RPMetaRest::DSC,
                KdoOp::ArrayUpdateContinue => {
                    let row_id = wrapper.from_rows.unwrap() as usize;
                    let begin_column = <[u16]>::ref_from_bytes(&v[0].data)?[row_id];

                    let data_sizes = <[u16]>::ref_from_bytes(&v[1].data)?;
                    let data_size = data_sizes[wrapper.from_rows.unwrap() as usize] as usize;
                    let data_offset = data_sizes.iter().take(row_id).sum::<u16>() as usize;

                    let meta = FromBytes::ref_from_bytes(&self.rest)?;
                    RPMetaRest::ArrayUpdateContinue(RPMeta21Wrapper {
                        meta,
                        begin_column,
                        data: &v[2].data[data_offset..(data_offset + data_size)],
                    })
                }
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

    // context variables
    from_rows: Option<u8>,
}

impl RPMetaWrapper<'_> {
    pub fn rest(&self) -> Result<Option<RPMetaRest<'_>>, ConvertError> {
        self.meta.rest(self.v, self)
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

#[derive(Debug)]
pub struct RPMetaIRPWrapper<'a> {
    pub irp: &'a RPMetaIRP,
    v: &'a [VectorData],
}

impl RPMetaIRPWrapper<'_> {
    /// `VARCHAR(N)`: raw string bytes. 'AAAA' -> 41414141 (hex)
    /// `NUMBER`: [crate::oracle::types::get_number_from_bytes]
    pub fn col_data(&self, col: usize) -> Option<Option<&'_ [u8]>> {
        self.irp.is_col_null(col as _).map(|is_null| {
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
pub struct RPMetaDRP {
    pub slot: u16,
    pub tabn: i8,
    nulls: [u8],
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMetaLKR {
    pub slot: u16,
    pub tabn: i8,
    /// TODO ? what's this? seems like lock mode?
    ///
    /// Guess:
    ///
    /// 0. free (undo of `SELECT FOR UPDATE`)
    /// 1. ?
    /// 2. X (redo of `SELECT FOR UPDATE`)
    pub to: i8,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMetaQMD {
    pub tabn: i8,
    pub lock: i8,
    pub nrow: i8,
    _unknown_1: u8,
    slots: [i16],
}

impl RPMetaQMD {
    pub fn slots(&self) -> &[i16] {
        &self.slots[..(self.nrow as usize)]
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMeta19<'a> {
    pub row_count: u8,
    pub rows: Vec<RPMetaWrapper<'a>>,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMetaUpdate {
    pub flags: u8,
    pub lock: u8,
    pub ckix: u8,
    /// printed in [i32]
    pub tabn: i8,
    pub slot: u16,
    pub ncol: u8,
    pub nnew: u8,
    pub size: u16,
    pub vect: i16,
}

#[derive(Debug)]
pub struct RPMeta21Wrapper<'a> {
    pub meta: &'a RPMetaUpdate,
    pub begin_column: u16,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub struct RPMetaURPWrapper<'a> {
    pub meta: &'a RPMetaUpdate,
    pub cols: &'a [u16],
    pub data: Vec<&'a [u8]>,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RPMetaQMI {
    pub fb_flags: IRPFbFlags,
    pub lb: u8,
    pub column_count: u8,

    /// size(u8) + data for each column, in order. size is the actual data size, excluding padding.
    col_data: [u8],
}

#[derive(Debug)]
pub struct RPMetaQMIWrapper<'a> {
    pub meta: &'a RPMetaQMD,
    pub data: &'a RPMetaQMI,
    pub cols: Vec<Option<&'a [u8]>>,
    pub tl: usize,
}

pub enum RPMetaRest<'a> {
    IUR,
    IRP(RPMetaIRPWrapper<'a>),
    DRP(&'a RPMetaDRP),
    LKR(&'a RPMetaLKR),
    URP(RPMetaURPWrapper<'a>),
    ORP(RPMetaIRPWrapper<'a>),
    MFC,
    CFA,
    CKI,
    SKL,
    QMI(RPMetaQMIWrapper<'a>),
    QMD(&'a RPMetaQMD),
    ArrayUpdate(RPMeta19<'a>),
    DSC,
    ArrayUpdateContinue(RPMeta21Wrapper<'a>),
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

impl Display for IRPFbFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}{}{}",
            if self.k() { 'K' } else { '-' },
            if self.c() { 'C' } else { '-' },
            if self.h() { 'H' } else { '-' },
            if self.d() { 'D' } else { '-' },
            if self.f() { 'F' } else { '-' },
            if self.l() { 'L' } else { '-' },
            if self.p() { 'P' } else { '-' },
            if self.n() { 'N' } else { '-' },
        )
    }
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
                from_rows: None,
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
