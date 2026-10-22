use bitfield_struct::bitfield;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::{
    change::{ConvertError, Op, VectorData, cv::FromCV, layers::Xid, parse_impl},
    redo::{DBA, UBA},
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
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let data = FromBytes::ref_from_bytes(&v[0].data)?;
        Ok(Self {
            data,
            pdbuid: if data.flag.pdb_uid() {
                // The pdb uid is the *trailing* 4-byte vector. An optional
                // extent-map redo (36 bytes, e.g. `kteop`) may precede it,
                // so `vector#1` is not always the pdb uid.
                let v = v.last().ok_or(ConvertError::Validity {
                    reason: "ktudh redo misses the pdb uid vector".to_string(),
                })?;
                Some(FromBytes::ref_from_bytes(&v.data[..v.size as usize])?)
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
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let data = FromBytes::ref_from_bytes(&v[0].data)?;
        // cv.data[2] ???
        Ok(Self {
            data,
            data2: if data.flags & 0x02 != 0 {
                Some(FromBytes::ref_from_bytes(&v[1].data)?)
            } else {
                None
            },
            scn: if data.flags & 0x01 != 0 {
                // TODO from ???
                Some(FromBytes::ref_from_prefix(&v[2].data)?.0)
            } else {
                None
            },
        })
    }
}

// -----------------------------------------------------------------------------

/// ktudb redo
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct UndoBlockRedoData {
    pub size: u16,
    pub space: u16,
    pub flags: u16,
    /// #{AMND}
    unknown: [u8; 2],
    pub xid: Xid,
    pub seq: u16,
    pub rec: u8,
    /// #{AMND}
    unknown2: [u8; 1],
}

/// ktubl redo
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct UndoBlockRedoData2 {
    pub objn: u32,
    pub objd: u32,
    pub tsn: i32,
    pub dba: DBA,

    pub record_layer_code: u8,
    pub record_op_code: u8,

    pub slot: u8,
    pub rci: u8,

    pub flags: UndoBlockRedoData2Flag,
    pub wrap: u16,
    rest: [u8],
}

impl UndoBlockRedoData2 {
    pub fn rest(&self) -> Result<UndoBlockRedoData2Rest<'_>, ConvertError> {
        if self.flags.extended() {
            Ok(UndoBlockRedoData2Rest::Extended(FromBytes::ref_from_bytes(
                &self.rest,
            )?))
        } else {
            Ok(UndoBlockRedoData2Rest::Standard(FromBytes::ref_from_bytes(
                &self.rest,
            )?))
        }
    }
}

pub enum UndoBlockRedoData2Rest<'a> {
    Standard(&'a UndoBlockRedoData2CommonRest),
    Extended(&'a UndoBlockRedoData2RestExt),
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct UndoBlockRedoData2RestExt {
    pub flags2: u16,
    pub buext_idx: u16,
    pub common: UndoBlockRedoData2CommonRest,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct UndoBlockRedoData2CommonRest {
    pub prev_ctl_uba: UBA,
    pub prev_ctl_max_cmt_scn: Scn,
    pub prev_tx_cmt_scn: Scn,
    unknown: [u8; 4],
    pub txn_start_scn: Scn,
    pub prev_brb: u32,
    pub prev_bcl: u32,
    pub logon_user: u32,
}

#[bitfield(u16)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct UndoBlockRedoData2Flag {
    /// 0. Regular Redo
    /// 1. MBU - HEAD
    /// 2. MBU - TAIL
    /// 3. MBU - HEAD ??
    #[bits(2)]
    pub undo_type: u16,

    #[bits(1)]
    pub last_buffer_split: bool,

    /// true: ktubl redo  - Begin trans
    /// false: ktubu redo
    #[bits(1)]
    pub is_ktubl: bool,

    #[bits(1)]
    pub user_undo_done: bool,

    #[bits(1)]
    pub temp_object: bool,

    #[bits(1)]
    pub user_only: bool,

    #[bits(1)]
    pub tablespace_undo: bool,

    #[bits(1)]
    pub mbu_mid: bool,

    #[bits(1)]
    unknown: u16,

    ///  txn start scn:  0x0000000000000000  logon user: 0
    /// prev brb:  0x01000772  prev bcl:  0x00000000
    #[bits(1)]
    pub txn_start_scn: bool,

    /// true:  start from 28
    /// false: start from 24
    #[bits(1)]
    pub extended: bool,

    #[bits(4)]
    unknown2: u16,
}

#[derive(Debug)]
pub struct UndoBlockRedo<'a> {
    pub data: &'a UndoBlockRedoData,
    pub data2: &'a UndoBlockRedoData2,
    pub op: Option<Box<Op<'a>>>, // TODO trim Box ?
}

impl<'a> FromCV<'a> for UndoBlockRedo<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let data2 = FromBytes::ref_from_bytes(&v[1].data)?;
        Ok(Self {
            data: FromBytes::ref_from_bytes(&v[0].data)?,
            data2,
            op: parse_impl(data2.record_layer_code, data2.record_op_code, &v[2..])
                .ok() // TODO error handling?
                .map(Box::new),
        })
    }
}

// -----------------------------------------------------------------------------
