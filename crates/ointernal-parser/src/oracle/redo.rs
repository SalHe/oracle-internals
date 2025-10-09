use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::oracle::{scn::Scn, timestamp::OracleTimestamp};

#[derive(KnownLayout, Immutable, FromBytes, IntoBytes, Debug)]
#[repr(C)]
pub struct RedoBlockHead {
    pub magic_number: u32,
    pub id: u32,
    pub seq: u16,
    _unknown_1: u16,
    pub offset: u16,
    pub checksum: u16,
}

impl RedoBlockHead {
    pub const MAGIC_NUMBER: u32 = 0x2201;

    pub fn offset(&self) -> u16 {
        self.offset & 0x7FFF
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct RedoFileHead {
    pub head: RedoBlockHead,

    /// #{AMND}
    _unknown_1: [u8; 4],

    pub vsn: u32,
    pub db_id: u32,
    pub db_name: [u8; 8],
    pub control_seq: u32,
    pub file_size: u32,
    pub block_size: u32,
    pub file_number: u16,
    pub file_type: u16,
    pub activation_id: u32,

    /// #{AMND}
    _zeros_1: [u8; 36],

    pub description: [u8; 64],

    pub nab: u32,

    pub reset_logs_count: u32,
    pub reset_log_scn: Scn,
    pub hws: u32,
    pub thread: u16,

    /// #{AMND}
    _unknown_11: [u8; 2],

    pub low_scn: Scn,
    pub low_scn_timestamp: OracleTimestamp,

    pub next_scn: Scn,
    pub next_scn_timestamp: OracleTimestamp,

    pub eot: u8,
    pub dis: u8,
    pub zero_blocks: u8,
    pub format_id: u8,

    pub enabled_scn: Scn,
    pub enabled_scn_timestamp: OracleTimestamp,

    pub thread_closed_scn: Scn,
    pub thread_closed_scn_timestamp: OracleTimestamp,

    /// #{AMND}
    _unknown_4: [u8; 4],

    /// MiscFlags
    pub misc_flags: u32,

    pub terminal_recovery_stop_scn: Scn,
    pub terminal_recovery_timestamp: OracleTimestamp,

    pub enabled_threads_bitmap1: [u8; 8],
    pub most_recent_redo_scn: Scn,
    pub largest_lwn: u32,
    pub real_next_scn: Scn,

    pub standby_apply_delay_minutes: i32,
    pub prev_reset_logs_scn: Scn,
    pub prev_reset_logs_count: u32,

    pub misc_flags2: u32,

    /// #{AMND}
    _zeros_3: [u8; 4],

    pub standby_log_close_time: OracleTimestamp,
    pub enabled_threads_bitmap2: [u8; 124],

    pub thread_internal_enable_tid: u32,
    pub thread_internal_enable_seq: u32,
    pub thread_internal_enable_scn: Scn,

    pub log_key: [u8; 16],

    /// #{AMND}
    _unknown_9: [u8; 16],

    pub redo_key_flag: u16,
    /// #{AMND}
    _unknown_10: [u8; 6],

    pub dax_hwm_block_number: u32,
    pub dax_flags: u16,
    pub dax_checksum: u16,

    /// #{AMND}
    trailing: [u8],
}

impl RedoFileHead {
    pub const fn threads_count(&self) -> usize {
        (self.enabled_threads_bitmap1.len() + self.enabled_threads_bitmap2.len()) * 8
    }

    pub fn is_thread_enabled(&self, i: usize) -> bool {
        let (bit_offset, bitmap) = if i / 8 < self.enabled_threads_bitmap1.len() {
            (i, &self.enabled_threads_bitmap1[..])
        } else {
            (
                i - self.enabled_threads_bitmap1.len() * 8,
                &self.enabled_threads_bitmap2[..],
            )
        };

        (bitmap[bit_offset / 8] >> (bit_offset % 8)) & 0x1 == 1
    }
}

bitflags! {
    pub struct MiscFlags: u32 {
        /// End-of-redo stream
        const END_OF_STREAM = 0x8;

        /// Archivelog created using asynchronous network transmittal
        const ASYNC_ARCH = 0x100;

        /// No data-loss mode
        const NO_LOSS_DATA_MODE = 0x200;

        /// Resynchronization mode
        const RESYNCHRONIZATION_MODE = 0x800;

        /// Closed thread archival
        const CLOSE_THREAD_ARCHIVAL = 0x1000;

        /// Maximize performance mode
        const MAX_PERFORMANCE_MODE = 0x2000;

        /// Failover
        const FAILOVER = 0x10000;

        /// Sync NoAffirm No data-loss mode
        const SYNC_NO_AFFIRM_NO_DATA_LOSS_MODE = 0x40000;

        const _ = !0;
    }
}

impl MiscFlags {
    pub fn protected_mode(&self) -> String {
        if self.contains(MiscFlags::NO_LOSS_DATA_MODE) {
            return "No data-loss mode".into();
        }

        let mut mode = Vec::new();
        if self.contains(MiscFlags::SYNC_NO_AFFIRM_NO_DATA_LOSS_MODE) {
            mode.push("Sync NoAffirm No data-loss mode");
        } else if self.contains(MiscFlags::RESYNCHRONIZATION_MODE) {
            mode.push("Resynchronization mode");
        } else {
            mode.push("Unprotected mode");
            if self.contains(MiscFlags::MAX_PERFORMANCE_MODE) {
                mode.push("Maximize performance mode");
            }
        }

        if self.contains(MiscFlags::CLOSE_THREAD_ARCHIVAL) {
            mode.push("Closed thread archival");
        }

        if self.contains(MiscFlags::ASYNC_ARCH) {
            mode.push("Archivelog created using asynchronous network transmittal");
        }

        mode.join(" + ")
    }
}

bitflags! {
    pub struct VldFlags: u8 {
        const KCRVALID = 0x01;
        const KCRDEPND = 0x02;
        const KCRVOID = 0x04;
        const KCRNMARK = 0x08;
        const KCRORDER = 0x10;
        const _ = !0;
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct RedoRecordHeadCommon {
    pub len: u32,
    pub vld: u8,

    /// #{AMND} padding?
    _unknown_1: u8,

    /// scn consists of scn_byte0..8
    scn_byte4: u8,
    scn_byte5: u8,

    scn_byte0: u8,
    scn_byte1: u8,
    scn_byte2: u8,
    scn_byte3: u8,

    pub sub_scn: u16,
    scn_byte6: u8,
    scn_byte7: u8,

    pub con_uid: u32,

    /// #{AMND} like padding?
    _unknown_2: [u8; 4],
}

impl RedoRecordHeadCommon {
    pub fn scn(&self) -> Scn {
        let bytes = [
            self.scn_byte0,
            self.scn_byte1,
            self.scn_byte2,
            self.scn_byte3,
            self.scn_byte4,
            self.scn_byte5,
            self.scn_byte6,
            self.scn_byte7,
        ];
        Scn::read_from_bytes(&bytes).unwrap()
    }
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct RedoRecordHeadV2 {
    pub common: RedoRecordHeadCommon,
    _unknown_1: [u8; 40],
    pub begin_time: OracleTimestamp,
}

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(C)]
pub struct RedoRecordHeadV1 {
    pub common: RedoRecordHeadCommon,
}

#[derive(KnownLayout, Immutable, FromBytes, Clone, Copy)]
#[repr(transparent)]
pub struct DBA(pub u32);

impl DBA {
    pub fn block_id(&self) -> u32 {
        self.0 & 0x003F_FFFF
    }

    pub fn relative_file_number(&self) -> u32 {
        (self.0 & 0xFFC0_0000) >> 22
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
