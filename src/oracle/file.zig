pub const FileHeader = extern struct {
    const MAGIC_NUMBER: u32 = 0x7A7B7C7D;

    _unknown_1: [20]u8,
    blockSize: u32,
    blockCount: u32,
    magicNumber: u32,
    _unknown_2: u32,
};

pub const Block0 = extern struct {
    header: FileHeader,
    _zeros: [0]u8,
};
