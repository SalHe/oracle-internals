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
contentOffset: u32 = 0,
offsetInBlock: u32 = 0,

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

    try self.nextRedoBlock();
    const redoBlock: *RedoFileContentBlock = @ptrCast(self.currBlock.?.ptr);
    if (redoBlock.head.offset.value != @sizeOf(redo.RedoBlockHead)) {
        // 说明是一个跨block的redo，理论上应该通过readChange将其获取出来，
        // 根据目前维护的接口来说，应该不会走到这里
        unreachable;
    }

    const record: *redo.RedoRecordHeadV2 = @ptrCast(&redoBlock.data);
    self.offsetInBlock += @sizeOf(redo.RedoRecordHeadV2);
    self.contentOffset = @sizeOf(redo.RedoRecordHeadV2);
    self.currRedoRecord = .{ .v2 = record.* };
    return self.currRedoRecord.?;
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
        .v1 => return ReadRedoError.NotImplemented,
        .v2 => |v2| len = v2.len,
    }

    if (self.contentOffset >= len) {
        self.inRecord = false;
        return ReadChangeError.NoMoreChange;
    }

    // change meta info
    const change: *redo.Change = @ptrCast(@alignCast(self.currBlock.?[self.offsetInBlock..]));

    // vector data size
    const fixedCost = paddingSize(change.changeListLen, 4) + @offsetOf(redo.Change, "changeListLen");
    self.contentOffset += fixedCost;
    self.offsetInBlock += fixedCost;

    var vectors = try std.ArrayList([]align(4) u8).initCapacity(vectorDataAllocator, change.vectorCount());
    errdefer vectors.deinit(vectorDataAllocator);

    const vectorSize = try self.allocator.alloc(u16, change.vectorCount());
    defer self.allocator.free(vectorSize);
    @memcpy(vectorSize, @as([*]u16, @ptrCast(&change.changeVector)));

    for (0..change.vectorCount()) |idx| {
        const currCost = paddingSize(vectorSize[idx], 4);

        const data: []align(4) u8 = try vectorDataAllocator.alignedAlloc(u8, .@"4", currCost);
        errdefer vectorDataAllocator.free(data);

        var crossed = false;
        if (self.offsetInBlock + currCost >= self.fileHeader().blockSize) {
            var rest = currCost;
            while (rest > 0) {
                const canRead = @min(self.fileHeader().blockSize - self.offsetInBlock, rest);
                @memcpy(data[currCost - rest .. currCost - rest + canRead], self.currBlock.?[self.offsetInBlock .. self.offsetInBlock + canRead]);
                rest -= canRead;
                self.offsetInBlock += canRead;

                if (self.offsetInBlock == self.fileHeader().blockSize) {
                    try self.nextRedoBlock();
                }
            }

            crossed = true;
        } else {
            @memcpy(data, self.currBlock.?[self.offsetInBlock .. self.offsetInBlock + currCost]);
            self.offsetInBlock += currCost;
        }

        try vectors.append(
            vectorDataAllocator,
            data,
        );
        self.contentOffset += currCost;
    }

    return .{
        .change = change.*,
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
