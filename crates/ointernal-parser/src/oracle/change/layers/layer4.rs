use bitfield_struct::bitfield;
use zerocopy::{FromBytes, Immutable, KnownLayout, Unalign};

use crate::oracle::{change::ConvertError, scn::Scn};

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
    scn_byte6: u8,
    scn_byte7: u8,
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
            self.scn_byte6,
            self.scn_byte7,
        ];
        Scn::read_from_bytes(&bytes).unwrap()
    }
}

// TODO should place here?
pub struct BlockCleanOutRecord<'a> {
    pub head: &'a BlockCleanOutRecordHead,
    pub entries: &'a [BlockCleanOutRecordEntry],
    pub spare: u32,
}

impl<'a> BlockCleanOutRecord<'a> {
    pub fn parse(rest: &'a [u8]) -> Result<Self, ConvertError> {
        let (head, rest) = BlockCleanOutRecordHead::ref_from_prefix(rest)?;
        let expected_size = std::mem::size_of::<BlockCleanOutRecordEntry>()
            * (head.entries_count as usize)
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

        let (entries, rest) = FromBytes::ref_from_prefix_with_elems(rest, head.entries_count as _)?;
        let (spare, _) = Unalign::<u32>::ref_from_prefix(rest)?;
        Ok(BlockCleanOutRecord {
            head,
            entries,
            spare: spare.get(),
        })
    }
}

// -----------------------------------------------------------------------------
