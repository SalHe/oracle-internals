use bitfield_struct::bitfield;
use num_derive::FromPrimitive;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{ChangeAndVector, ConvertError, cv::FromCV, layers::Xid},
    redo::{DBA, UBA},
};

// -----------------------------------------------------------------------------

/// KTB Redo
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct InsertRowPieceOp {
    pub b1: IRPByte1,
    pub b2: IRPByte2,
    padding: [u8; 2],
    rest: [u8],
}

impl InsertRowPieceOp {
    pub fn rest(&self) -> Result<&InsertRowPieceOpRest, ConvertError> {
        let rest = if self.b2.start_from_8() {
            InsertRowPieceOpRest::ref_from_bytes(&self.rest[4..])?
        } else {
            InsertRowPieceOpRest::ref_from_bytes(&self.rest)?
        };
        Ok(rest)
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct InsertRowPieceOpRest {
    pub xid: Xid,
    pub uba: UBA,
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

impl IRPByte1 {
    pub fn op_ch(&self) -> Option<char> {
        match self.op() {
            1 => Some('F'),
            2 => Some('C'),
            3 => Some('Z'),
            4 => Some('L'),
            5 => Some('R'),
            6 => Some('N'),
            8 => Some('G'),
            _ => None,
        }
    }
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
pub struct InsertRowPieceMeta {
    pub bdba: DBA,
    pub hdba: DBA,

    pub maxfr: u16,
    pub byte1: IRPMetaByte1,
    pub xtype: IRPMetaByte2,

    pub itli: u8,
    pub ispac: u8,
    _unknown_1: [u8; 2],

    /*
    pub fwd: u32,
    pub fwd_pos: u16,
    pub bkw_low: u16,
    pub bkw_high: u16,
    pub bkw_pos: u16,

    pub flags: IRPMetaFlags,
    pub lock: u8,
    pub slot: u8,
    _unknown_2: [u8; 9],
    */
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
    rest: [u8],
}

impl InsertRowPieceMeta {
    // pub fn fwd(&self) -> u32 {
    //     (self.bkw_high as u32) << 16 | self.bkw_low as u32
    // }

    pub fn is_col_null(&self, col: u8) -> Option<bool> {
        if col < self.column_count {
            Some(self.rest[(col as usize) / 8] >> (col % 8) != 0)
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
    SHK = 0x14,
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
pub struct InsertRowPiece<'a> {
    pub op: &'a InsertRowPieceOp,
    pub meta: &'a InsertRowPieceMeta,
}

impl<'a> FromCV<'a> for InsertRowPiece<'a> {
    fn from_cv(cv: &'a ChangeAndVector) -> Result<Self, ConvertError> {
        // TODO columns data from vector[2..]
        // TODO nulls?
        Ok(Self {
            op: FromBytes::ref_from_bytes(&cv.vector[0])?,
            meta: FromBytes::ref_from_bytes(&cv.vector[1])?,
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
