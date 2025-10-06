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
    if (self.currBlock != null) {
        self.allocator.free(self.currBlock.?);
        self.currBlock = null;
    }

    try self.readFileHeader();
    try self.readRedoHeader();
    self.currBlock = try self.readBlock();
}

fn readBlock(self: *RedoReader) ![]align(4) u8 {
    const buffer = try self.allocator.alignedAlloc(u8, .@"4", self.fileHeader().blockSize);
    errdefer self.allocator.free(buffer);
    _ = try self.file.readAll(buffer);
    // TODO checksum
    return buffer;
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

pub fn beginReadRecord(self: *RedoReader) ReadRedoError!redo.RedoRecordHead {
    if (self.inRecord) {
        return ReadRedoError.AlreadyInRecord;
    }

    self.inRecord = true;
    self.contentOffset = 0;
    self.offsetInBlock = @sizeOf(redo.RedoBlockHead);

    const redoBlock: *RedoFileContentBlock = @ptrCast(self.currBlock.?.ptr);
    if (redoBlock.head.offset.value != @sizeOf(redo.RedoBlockHead)) {
        // TODO last redo
        std.debug.print("Warning: redoBlock.head.offset != 0\n", .{});
    }

    const record: *redo.RedoRecordHeadV2 = @ptrCast(&redoBlock.data);
    self.offsetInBlock += @sizeOf(redo.RedoRecordHeadV2);
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
        return ReadChangeError.NoMoreChange;
    }

    // change meta info
    const change: *redo.Change = @ptrCast(@alignCast(self.currBlock.?[self.offsetInBlock..]));

    // vector data size
    const fixedCost = paddingSize(change.changeListLen, 4) + @offsetOf(redo.Change, "changeListLen");
    self.contentOffset += fixedCost;
    self.offsetInBlock += fixedCost; // TODO cross block?

    var vectors = try std.ArrayList([]align(4) u8).initCapacity(vectorDataAllocator, change.vectorCount());
    errdefer vectors.deinit(vectorDataAllocator);

    for (0..change.vectorCount()) |idx| {
        const vector: [*]u16 = @ptrCast(&change.changeVector);
        const currCost = paddingSize(vector[idx], 4);
        self.contentOffset += currCost;

        if (self.offsetInBlock + currCost >= self.fileHeader().blockSize) {
            // self.currBlock = try self.readBlock();
            self.offsetInBlock = 0;
        } else {
            const data: []align(4) u8 = try vectorDataAllocator.alignedAlloc(u8, .@"4", currCost);
            errdefer vectorDataAllocator.free(data);
            const src = self.currBlock.?[self.offsetInBlock .. self.offsetInBlock + currCost];
            @memcpy(data, src);
            try vectors.append(
                vectorDataAllocator,
                data,
            );

            self.offsetInBlock += currCost;
        }
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
