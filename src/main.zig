const std = @import("std");
const oracle_internals = @import("oracle_internals");
const oracle = oracle_internals.oracle;
const FileHeader = oracle_internals.oracle.file.FileHeader;
const Block0 = oracle_internals.oracle.file.Block0;
const redo = oracle_internals.oracle.redo;

pub fn main() !void {
    // allocator
    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena.deinit();

    const allocator = arena.allocator();

    // input redo file
    const args = std.process.argsAlloc(allocator) catch {
        std.debug.print("Failed to get args\n", .{});
        return;
    };
    defer std.process.argsFree(allocator, args);
    if (args.len < 2) {
        std.debug.print("Usage: {s} <oracle_redo_file>\n", .{args[0]});
        return;
    }

    // read redo file
    const filePath = args[1];
    var reader = try oracle_internals.oracle.RedoReader.init(allocator, filePath);
    defer reader.deinit();
    std.debug.print("File Header: {any}\n", .{reader.fileHeader()});
    std.debug.print("Redo Header: {any}\n", .{reader.redoHeader()});

    var iRecord: u32 = 0;
    while (true) {
        const record = reader.beginReadRecord() catch |err| {
            if (err == oracle.RedoReader.ReadRedoError.NoMoreRedo) {
                break;
            } else {
                return err;
            }
        };
        std.debug.print("\n", .{});
        std.debug.print("Record#{}: {any}\n", .{ iRecord, record });
        iRecord += 1;

        var iChange: u32 = 0;
        while (true) {
            var change = reader.readChange(allocator) catch |err| {
                if (err == oracle.RedoReader.ReadChangeError.NoMoreChange) {
                    break;
                } else {
                    return err;
                }
            };
            defer change.deinit(allocator);

            std.debug.print("Change#{}: {any}\n", .{ iChange, change });
            for (change.vectors.items, 0..) |vec, j| {
                std.debug.print("  Vector#{}: {any}\n", .{ j, vec });
            }
            iChange += 1;
        }
    }
}
