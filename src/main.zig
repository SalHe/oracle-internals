const std = @import("std");
const oracle_internals = @import("oracle_internals");
const FileHeader = oracle_internals.oracle.file.FileHeader;
const Block0 = oracle_internals.oracle.file.Block0;
const redo = oracle_internals.oracle.redo;

pub fn main() !void {}

test "read redo file" {
    // test allocator
    const allocator = std.testing.allocator;

    const filePath = "/home/salhe/projects/oracle-internals/temp/1_46_1213673283.dbf";
    var reader = try oracle_internals.oracle.RedoReader.init(allocator, filePath);
    defer reader.deinit();
    std.debug.print("File Header: {any}\n", .{reader.fileHeader()});
    std.debug.print("Redo Header: {any}\n", .{reader.redoHeader()});

    const record = try reader.beginReadRecord();
    std.debug.print("First Record: {any}\n", .{record});

    for (0..6) |i| {
        var change = try reader.readChange(allocator);
        defer change.deinit(allocator);

        std.debug.print("Change#{}: {any}\n", .{ i, change });
        for (change.vectors.items, 0..) |vec, j| {
            std.debug.print("  Vector#{}: {any}\n", .{ j, vec });
        }
    }
}
