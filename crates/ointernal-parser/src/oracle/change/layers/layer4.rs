use bitfield_struct::bitfield;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unalign};

use crate::oracle::{
    change::{ConvertError, VectorData, cv::FromCV},
    scn::Scn,
};

// -----------------------------------------------------------------------------

#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct BlockCleanOutRecordHead {
    /// #{AMND}
    unknown_1: [u8; 20],
    pub opt: BlockCleanOutRecordOpt,
    pub entries_count: u8,
    pub flags: BlockCleanOutRecordFlag,
    unknown_2: u8,
    pub scn: Scn, // 0x04ac
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct BlockCleanOutRecordOpt {
    #[bits(4)]
    unknown_1: u8,
    contains_entries: bool,
    #[bits(3)]
    unknown_2: u8,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct BlockCleanOutRecordFlag {
    #[bits(2)]
    pub version: u8,
    #[bits(1)]
    pub big_scn: bool,
    #[bits(1)]
    pub compact: bool,

    /// #{AMND}
    #[bits(4)]
    unknown_1: u8,
}

/// One cleanout entry (8 bytes). The SCN is stored as a 6-byte
/// `{base: u32, wrapper: u16}` pair rather than the usual 8-byte [`Scn`].
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct BlockCleanOutRecordEntry {
    pub itli: u8,
    pub flag: BlockCleanOutRecordEntryFlag,
    scn_byte4: u8,
    scn_byte5: u8,
    scn_byte0: u8,
    scn_byte1: u8,
    scn_byte2: u8,
    scn_byte3: u8,
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct BlockCleanOutRecordEntryFlag {
    #[bits(2)]
    pub opt: u8,
    #[bits(6)]
    pub whr: u8,
}

impl BlockCleanOutRecordEntry {
    pub fn scn(&self) -> Scn {
        let bytes = [
            self.scn_byte0,
            self.scn_byte1,
            self.scn_byte2,
            self.scn_byte3,
            self.scn_byte4,
            self.scn_byte5,
            0,
            0,
        ];
        Scn::read_from_bytes(&bytes).unwrap()
    }
}

/// Standalone `OP:4.1` records omit the 20-byte prefix that embedded cleanout
/// records (layer 11 `op: F`) carry.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct BlockCleanOutRecordTailHead {
    pub opt: BlockCleanOutRecordOpt,
    pub entries_count: u8,
    pub flags: BlockCleanOutRecordFlag,
    unknown_1: u8,
    pub scn: Scn,
}

// TODO should place here?
#[derive(Debug)]
pub struct BlockCleanOutRecord<'a> {
    pub opt: BlockCleanOutRecordOpt,
    pub flags: BlockCleanOutRecordFlag,
    pub scn: Scn,
    pub entries: &'a [BlockCleanOutRecordEntry],
    pub spare: u32,
}

impl<'a> BlockCleanOutRecord<'a> {
    /// Parses a cleanout record embedded in a layer 11 change.
    pub fn parse(rest: &'a [u8]) -> Result<Self, ConvertError> {
        let (head, rest) = BlockCleanOutRecordHead::ref_from_prefix(rest)?;
        Self::parse_body(head.opt, head.flags, head.scn, head.entries_count, rest)
    }

    /// Parses a standalone `OP:4.1` record, which has no prefix.
    pub fn parse_standalone(data: &'a [u8]) -> Result<Self, ConvertError> {
        let (head, rest) = BlockCleanOutRecordTailHead::ref_from_prefix(data)?;
        Self::parse_body(head.opt, head.flags, head.scn, head.entries_count, rest)
    }

    fn parse_body(
        opt: BlockCleanOutRecordOpt,
        flags: BlockCleanOutRecordFlag,
        scn: Scn,
        entries_count: u8,
        rest: &'a [u8],
    ) -> Result<Self, ConvertError> {
        // Layout: `entries_count` 8-byte entries, then two unknown bytes per
        // entry, then a 4-byte spare.
        let count = entries_count as usize;
        let expected_size = std::mem::size_of::<BlockCleanOutRecordEntry>() * count
            + 2 * count
            + std::mem::size_of::<u32>();
        if rest.len() < expected_size {
            return Err(ConvertError::Validity {
                reason: format!(
                    "block cleanout record content length is not enough, should >= {}, but is {}",
                    expected_size,
                    rest.len(),
                ),
            });
        }

        let (entries, rest) = FromBytes::ref_from_prefix_with_elems(rest, count)?;
        // `#{AMND}`: zero in every observed record.
        let (_unknown, rest) = rest.split_at(2 * count);
        let (spare, _) = Unalign::<u32>::ref_from_prefix(rest)?;
        Ok(BlockCleanOutRecord {
            opt,
            flags,
            scn,
            entries,
            spare: spare.get(),
        })
    }
}

impl<'a> FromCV<'a> for BlockCleanOutRecord<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        // The whole OP:4.1 record is a single vector.
        let data = &v[0].data[..v[0].size as usize];
        Self::parse_standalone(data)
    }
}

// -----------------------------------------------------------------------------
