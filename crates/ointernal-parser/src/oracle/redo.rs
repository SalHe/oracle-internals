use std::mem::transmute;

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, KnownLayout};

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RedoBlockHead {
    pub magic_number: u32,
    pub id: u32,
    pub seq: u32,
    pub offset: u16,
    pub checksum: u16,
}

impl RedoBlockHead {
    pub fn offset(&self) -> u16 {
        self.offset & 0x7FFF
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct Scn {
    pub base: u32,
    pub wrapper: u16,
    pub _unknown: u16,
}

impl Scn {
    pub fn to_u64(self) -> u64 {
        self.into()
    }
}

impl Into<u64> for Scn {
    fn into(self) -> u64 {
        unsafe { transmute(self) }
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct OracleTimestamp(pub u32);

impl OracleTimestamp {
    pub fn real_timestamp(&self) -> u32 {
        self.0 + 545961600
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RedoFileHead {
    pub head: RedoBlockHead,
    _unknown_1: [u8; 4],
    pub vsn: u32,
    pub db_id: u32,
    pub db_name: [u8; 8],
    pub control_seq: u32,
    _unknown_7: [u8; 8],
    pub file_number: u16,
    pub file_type: u16,
    pub activation_id: u32,
    _zeros_1: [u8; 36],
    pub description: [u8; 64],

    pub nab: u32,

    pub reset_logs_count: u32,
    pub reset_log_scn: Scn,
    pub hws: u8,
    _unknown_2: [u8; 7],

    pub low_scn: Scn,
    pub low_scn_timestamp: OracleTimestamp,

    pub next_scn: Scn,
    pub next_scn_timestamp: OracleTimestamp,

    _unknown_3: [u8; 4],

    pub enabled_scn: Scn,
    pub enabled_scn_timestamp: OracleTimestamp,

    pub thread_closed_scn: Scn,
    pub thread_closed_scn_timestamp: OracleTimestamp,

    _unknown_4: [u8; 4],

    pub misc_flags: u32,
    pub misc_flags2: u32,

    _unknown_5: [u8; 28],
    pub real_next_scn: Scn,

    _unknown_6: [u8; 4],
    pub prev_reset_logs_scn: Scn,
    pub prev_reset_logs_count: u32,

    _zeros_2: [u8; 152],

    pub log_key: [u8; 16],
    trailing: [u8],
}

bitflags! {
    pub struct VldFlags: u8 {
        const KCRVALID = 0x01;
        const KCRDEPND = 0x02;
        const KCRVOID = 0x04;
        const KCRNMARK = 0x08;
        const KCRORDER = 0x10;
        const UNKNOWN1 = 0x20;
        const UNKNOWN2 = 0x40;
        const UNKNOWN3 = 0x80;
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct RedoRecordHeadCommon {
    pub len: u32,
    pub vld: u8,
    _unknown_1: u8,
    pub sub_scn: u16,
    pub scn: Scn,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct RedoRecordHeadV2 {
    pub common: RedoRecordHeadCommon,
    _unknown_2: [u8; 48],
    pub begin_time: OracleTimestamp,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct RedoRecordHeadV1 {
    pub common: RedoRecordHeadCommon,
    _unknown_2: [u8; 8],
}

#[derive(KnownLayout, Immutable, FromBytes, Clone, Copy)]
#[repr(transparent)]
pub struct DBA(u32);

impl DBA {
    pub fn block_id(&self) -> u32 {
        self.0 & 0x003F_FFFF
    }

    pub fn relative_file_number(&self) -> u32 {
        self.0 & 0xFFC0_0000
    }
}

impl std::fmt::Debug for DBA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DBA(block_id: {}, file_number: {})={:x}",
            self.block_id(),
            self.relative_file_number(),
            self.0,
        )
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct Change {
    pub layer: u8,
    pub op: u8,
    pub block_class: u16,
    pub file_number: u16,
    pub obj_high: u16,
    pub dba: DBA,
    pub scn: Scn,

    /// SEQ(Sequence number relative to SCN)
    pub seq: u8,
    pub type_: u8,
    pub obj_low: u16,
    _unknown_1: [u8; 8],
    pub change_list_len: u16,
    pub first_vector_size: u16,
    // rest change vector
}

impl Change {
    pub fn obj(&self) -> u32 {
        ((self.obj_high as u32) << 16) | self.obj_low as u32
    }

    pub fn vector_count(&self) -> u16 {
        self.change_list_len / 2 - 1
    }

    pub fn need_padding(&self) -> bool {
        if self.vector_count() == 0 {
            // consumed by [first_vector_size]
            return false;
        }

        return self.vector_count() % 2 == 0;
    }
}
