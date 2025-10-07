const std = @import("std");

pub const RedoBlockHead = extern struct {
    const MAGIC_NUMBER = 0x00002201;

    magicNumber: u32,
    id: u32,
    sequence: u32,

    offset: packed struct(u16) {
        value: u15,
        _unknown_1: u1,
    },

    checksum: u16,
};

pub const Scn = extern struct {
    base: u32,
    wrapper: u16,
    _unknown: [2]u8,
};

pub const OracleTimestamp = u32;
pub const ORACLE_TIMESTAMP_BASE = 545961600; // 1987-01-01 00:00:00 UTC

pub const RedoFileHeadBlock = extern struct {
    head: RedoBlockHead,
    _unknown_1: [4]u8,
    vsn: u32,
    dbId: u32,
    dbName: [8]u8,
    controlSeq: u32,
    _unknown_7: [8]u8,
    fileNumber: u16,
    fileType: u16,
    activationId: u32,
    _zeros_1: [36]u8,
    description: [64]u8,

    nab: u32,

    resetLogsCount: u32,
    resetLogScn: Scn,
    hws: u8,
    _unknown_2: [7]u8,

    lowScn: Scn,
    lowScnTimestamp: OracleTimestamp,

    nextScn: Scn,
    nextScnTimestamp: OracleTimestamp,

    _unknown_3: [4]u8,

    enabledScn: Scn,
    enabledScnTimestamp: OracleTimestamp,

    threadClosedScn: Scn,
    threadClosedScnTimestamp: OracleTimestamp,

    _unknown_4: [4]u8,

    miscFlags: u32,
    miscFlags2: u32,

    _unknown_5: [28]u8,
    realNextScn: Scn,

    _unknown_6: [4]u8,
    prevResetLogsScn: Scn,
    prevResetLogsCount: u32,

    _zeros_2: [152]u8,

    logKey: [16]u8,

    tailing: [0]u8,
};

pub const RedoFileContentBlock = extern struct {
    head: RedoBlockHead,
    data: [0]u8,
};

pub const RedoRecordHeadCommon = extern struct {
    len: u32,
    vld: packed struct(u8) {
        KCRVALID: u1,
        KCRDEPND: u1,
        KCRVOID: u1,
        KCRNMARK: u1,
        KCRORDER: u1,
        _unknown_0: u3,
    },
    _unknown_1: u8,
    subScn: u16,
    scn: Scn,
};

pub const RedoRecordHeadV2 = extern struct {
    common: RedoRecordHeadCommon,
    _unknown_2: [48]u8,
    beginTime: OracleTimestamp,
    data: [0]u8,
};

pub const RedoRecordHeadV1 = extern struct {
    common: RedoRecordHeadCommon,
    _unknown_2: [8]u8,
};

pub const RedoRecordHead = union(enum) {
    v1: RedoRecordHeadV1,
    v2: RedoRecordHeadV2,
};

pub const DBA = packed struct(u32) {
    fileNumber: u22,
    blockNumber: u10,
};

pub const Change = extern struct {
    layer: u8,
    op: u8,
    blockClass: u16,
    fileNumber: u16,
    objHigh: u16,
    dba: DBA,
    scn: Scn,

    /// SEQ(Sequence number relative to SCN)
    seq: u8,
    type: u8,
    objLow: u16,
    _unknown_1: [8]u8,
    changeListLen: u16,
    changeVector: [0]u16,

    pub fn obj(self: *Change) u32 {
        return (u32(self.objHigh) << 16) | u32(self.objLow);
    }

    pub fn vectorCount(self: *Change) u32 {
        return self.changeListLen / 2 - 1;
    }
};

pub const ChangeAndVector = struct {
    change: Change,
    vectors: std.ArrayList([]align(4) u8),

    pub fn deinit(self: *ChangeAndVector, allocator: std.mem.Allocator) void {
        for (self.vectors.items) |vec| {
            allocator.free(vec);
        }
        self.vectors.deinit(allocator);
    }
};
