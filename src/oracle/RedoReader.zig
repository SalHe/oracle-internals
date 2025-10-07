const std = @import("std");
const RedoReader = @This();
const oracle_internals = @import("../root.zig");
const redo = oracle_internals.oracle.redo;
const RedoFileContentBlock = redo.RedoFileContentBlock;
const RedoFileHeadBlock = redo.RedoFileHeadBlock;
const FileHeader = oracle_internals.oracle.file.FileHeader;
const Block0 = oracle_internals.oracle.file.Block0;

allocator: std.mem.Allocator,
file: std.fs.File,

/// Block#0
_fileHeader: ?[]align(4) u8,

/// Block#1
_redoHeader: ?[]align(4) u8,

inRecord: bool = false,
currBlock: ?[]align(4) u8 = null,
currRedoRecord: ?redo.RedoRecordHead = null,
contentOffset: usize = 0,
offsetInBlock: usize = 0,

pub const ReadRedoError = error{
    NotOracleRedoFile,
    AlreadyInRecord,
    NotInRecord,
    NotImplemented,
    NoMoreRedo,
};

pub const ReadChangeError = error{
    NoMoreChange,
} || ReadRedoError;

pub fn init(allocator: std.mem.Allocator, path: []const u8) !RedoReader {
    var reader = RedoReader{
        .allocator = allocator,
        .file = try std.fs.cwd().openFile(path, .{ .mode = .read_only }),
        ._fileHeader = null,
        ._redoHeader = null,
    };
    errdefer reader.file.close();

    try reader.reloadHeader();
    return reader;
}

pub fn deinit(self: *RedoReader) void {
    self.file.close();

    if (self._fileHeader != null) {
        self.allocator.free(self._fileHeader.?);
        self._fileHeader = null;
    }

    if (self._redoHeader != null) {
        self.allocator.free(self._redoHeader.?);
        self._redoHeader = null;
    }

    if (self.currBlock != null) {
        self.allocator.free(self.currBlock.?);
        self.currBlock = null;
    }
}

fn reloadHeader(self: *RedoReader) !void {
    try self.readFileHeader();
    try self.readRedoHeader();
    try self.nextRedoBlock();
}

fn readBlock(self: *RedoReader) ![]align(4) u8 {
    const buffer = try self.allocator.alignedAlloc(u8, .@"4", self.fileHeader().blockSize);
    errdefer self.allocator.free(buffer);
    const read = try self.file.readAll(buffer);
    if (read != self.fileHeader().blockSize) {
        return ReadRedoError.NoMoreRedo;
    }
    // TODO checksum
    return buffer;
}

fn nextRedoBlock(self: *RedoReader) !void {
    if (self.currBlock != null) {
        self.allocator.free(self.currBlock.?);
        self.currBlock = null;
    }

    self.currBlock = try self.readBlock();
    self.offsetInBlock = @sizeOf(redo.RedoBlockHead);
    std.debug.print("block id: {any}\n", .{self.blockHead().id});
}

fn blockHead(self: *RedoReader) *redo.RedoBlockHead {
    return @ptrCast(self.currBlock.?.ptr);
}

fn readFileHeader(self: *RedoReader) !void {
    if (self._fileHeader != null) {
        self.allocator.free(self._fileHeader.?);
        self._fileHeader = null;
    }

    var fileHead: FileHeader = undefined;
    try self.file.seekTo(0);
    _ = try self.file.readAll(@ptrCast(&fileHead));

    self._fileHeader = try self.allocator.alignedAlloc(u8, .@"4", fileHead.blockSize);
    errdefer self.allocator.free(self._fileHeader.?);

    try self.file.seekTo(0);
    _ = try self.file.readAll(self._fileHeader.?);
}

fn readRedoHeader(self: *RedoReader) !void {
    if (self._redoHeader != null) {
        self.allocator.free(self._redoHeader.?);
        self._redoHeader = null;
    }

    self._redoHeader = try self.readBlock();
    errdefer self.allocator.free(self._redoHeader.?);
}

pub fn fileHeader(self: *RedoReader) *FileHeader {
    return @ptrCast(self._fileHeader.?.ptr);
}

pub fn redoHeader(self: *RedoReader) *RedoFileHeadBlock {
    return @ptrCast(self._redoHeader.?.ptr);
}

pub fn beginReadRecord(self: *RedoReader) !redo.RedoRecordHead {
    if (self.inRecord) {
        return ReadRedoError.AlreadyInRecord;
    }

    self.inRecord = true;

    if (self.fileHeader().blockSize - self.offsetInBlock < @min(@sizeOf(redo.RedoRecordHeadV1), @sizeOf(redo.RedoRecordHeadV2))) {
        try self.nextRedoBlock();
    }

    while (true) {
        const redoBlock: *RedoFileContentBlock = @ptrCast(self.currBlock.?.ptr);
        const record: *redo.RedoRecordHeadCommon = @ptrCast(@alignCast(&(@as([*]u8, @ptrCast(&redoBlock.data))[self.offsetInBlock - @sizeOf(redo.RedoBlockHead)])));

        if (record.vld.KCRVOID == 1) {
            const redoV2: *redo.RedoRecordHeadV2 = @ptrCast(@alignCast(record));
            self.offsetInBlock += @sizeOf(redo.RedoRecordHeadV2);
            self.contentOffset = @sizeOf(redo.RedoRecordHeadV2);
            self.currRedoRecord = .{ .v2 = redoV2.* };

            if (redoV2.common.len == 0) {
                try self.nextRedoBlock();
                continue;
            }
        } else {
            const redoV1: *redo.RedoRecordHeadV1 = @ptrCast(@alignCast(record));
            self.offsetInBlock += @sizeOf(redo.RedoRecordHeadV1);
            self.contentOffset = @sizeOf(redo.RedoRecordHeadV1);
            self.currRedoRecord = .{ .v1 = redoV1.* };

            if (redoV1.common.len == 0) {
                try self.nextRedoBlock();
                continue;
            }
        }

        return self.currRedoRecord.?;
    }
}

fn readContent(self: *RedoReader, dest: []u8) !void {
    const record = self.currRedoRecord orelse {
        return ReadRedoError.NotInRecord;
    };

    var len: u32 = 0;
    switch (record) {
        .v1 => |v1| len = v1.common.len,
        .v2 => |v2| len = v2.common.len,
    }

    if (self.contentOffset >= len) {
        // TODO
    }

    if (self.offsetInBlock + dest.len >= self.fileHeader().blockSize) {
        var rest = dest.len;
        while (rest > 0) {
            const canRead = @min(self.fileHeader().blockSize - self.offsetInBlock, rest);
            @memcpy(dest[dest.len - rest .. dest.len - rest + canRead], self.currBlock.?[self.offsetInBlock .. self.offsetInBlock + canRead]);
            rest -= canRead;
            self.offsetInBlock += canRead;

            if (self.offsetInBlock == self.fileHeader().blockSize) {
                try self.nextRedoBlock();
            }
        }
    } else {
        @memcpy(dest, self.currBlock.?[self.offsetInBlock .. self.offsetInBlock + dest.len]);
        self.offsetInBlock += dest.len;
    }

    self.contentOffset += dest.len;
}

pub fn readChange(self: *RedoReader, vectorDataAllocator: std.mem.Allocator) !redo.ChangeAndVector {
    if (!self.inRecord) {
        return ReadRedoError.NotInRecord;
    }

    const record = self.currRedoRecord orelse {
        return ReadRedoError.NotInRecord;
    };

    var len: u32 = 0;
    switch (record) {
        .v1 => |v1| len = v1.common.len,
        .v2 => |v2| len = v2.common.len,
    }

    if (self.contentOffset >= len) {
        self.inRecord = false;
        return ReadChangeError.NoMoreChange;
    }

    // save change meta info(when cross block, ptr freed)
    var change: redo.Change = undefined;
    try self.readContent(@as([*]u8, @ptrCast(&change))[0..(@sizeOf(@TypeOf(change.changeListLen)) + @offsetOf(redo.Change, "changeListLen"))]);
    var vectorSize: []u16 = undefined;
    {
        // save vector size
        vectorSize = try self.allocator.alloc(u16, change.vectorCount());
        try self.readContent(@as([*]u8, @ptrCast(&vectorSize[0]))[0..(vectorSize.len * @sizeOf(u16))]);
    }
    defer self.allocator.free(vectorSize);

    if (change.vectorCount() % 2 == 0) {
        self.contentOffset += 2;
        self.offsetInBlock += 2;
    }

    // read vector data
    var vectors: std.ArrayList([]align(4) u8) = undefined;
    vectors = try std.ArrayList([]align(4) u8).initCapacity(vectorDataAllocator, change.vectorCount());
    errdefer vectors.deinit(vectorDataAllocator);
    for (0..change.vectorCount()) |idx| {
        const currCost = paddingSize(vectorSize[idx], 4);

        const data: []align(4) u8 = try vectorDataAllocator.alignedAlloc(u8, .@"4", currCost);
        errdefer vectorDataAllocator.free(data);
        try self.readContent(data);

        try vectors.append(
            vectorDataAllocator,
            data,
        );
    }

    return .{
        .change = change,
        .vectors = vectors,
    };
}

fn paddingSize(size: u32, alignment: u32) u32 {
    if (size % alignment == 0) {
        return size;
    } else {
        return size + alignment - (size % alignment);
    }
}
