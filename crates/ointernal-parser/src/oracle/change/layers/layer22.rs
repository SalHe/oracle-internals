use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::change::{ConvertError, VectorData, cv::FromCV};

/// Disposition of the file space header save area (`ktfbhredo`, second byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSpaceHeaderSave {
    /// 0: printed as `Save: `.
    Save,
    /// 1: printed as `NoSave: `.
    NoSave,
    /// Any other value: printed as `Clear: `.
    Clear,
}

/// Pending file space header operation (`ktfbhredo`, first byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSpaceHeaderOp {
    /// 0
    NoPending,
    /// 1
    UsingExtent,
    /// 2
    FreeingExtent,
    /// 3
    FreeExtent,
    /// 4
    UseExtent,
    /// 5
    FileResize,
    /// 6
    StatsOn,
    /// 7
    FileAutoExtendOn,
    /// 8
    FileAutoExtendOff,
    /// 9
    StatsOff,
    /// 0x10
    InvalidateRange,
    /// Anything else.
    Unknown(u8),
}

impl FileSpaceHeaderOp {
    fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::NoPending,
            1 => Self::UsingExtent,
            2 => Self::FreeingExtent,
            3 => Self::FreeExtent,
            4 => Self::UseExtent,
            5 => Self::FileResize,
            6 => Self::StatsOn,
            7 => Self::FileAutoExtendOn,
            8 => Self::FileAutoExtendOff,
            9 => Self::StatsOff,
            0x10 => Self::InvalidateRange,
            other => Self::Unknown(other),
        }
    }
}

/// The fixed 12-byte layout of a file space header redo payload.
///
/// Shorter payloads carry only the first [`FileSpaceHeaderRedo::MIN_LEN`]
/// bytes. Oracle still reads the following record bytes as the missing
/// fields: the reader's 4-byte padding covers `instance`, while `begin` and
/// `length` come from unrelated bytes of the redo record (see the note).
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FileSpaceHeaderRedoHead {
    /// [`FileSpaceHeaderOp`] as a raw byte.
    pub op: u8,
    /// [`FileSpaceHeaderSave`] as a raw byte.
    pub save: u8,
    pub instance: u16,
    pub begin: u32,
    pub length: u32,
}

/// `ktfbhredo - File Space Header Redo:` (OP:22.2).
///
/// The payload is one vector of 2 or 12 bytes: the first two bytes are always
/// present; `instance` additionally needs the 4-byte padding, and
/// `begin`/`length` only exist when the vector is at least 12 bytes long
/// (which is the case for every operation except `NoPending`).
#[derive(Debug)]
pub struct FileSpaceHeaderRedo<'a> {
    data: &'a [u8],
}

impl FileSpaceHeaderRedo<'_> {
    pub const MIN_LEN: usize = 2;

    pub fn op(&self) -> FileSpaceHeaderOp {
        FileSpaceHeaderOp::from_raw(self.data[0])
    }

    pub fn save(&self) -> FileSpaceHeaderSave {
        match self.data[1] {
            0 => FileSpaceHeaderSave::Save,
            1 => FileSpaceHeaderSave::NoSave,
            _ => FileSpaceHeaderSave::Clear,
        }
    }

    pub fn instance(&self) -> u16 {
        self.word16(2)
    }

    pub fn begin(&self) -> u32 {
        self.word32(4)
    }

    pub fn length(&self) -> u32 {
        self.word32(8)
    }

    fn word16(&self, offset: usize) -> u16 {
        self.data
            .get(offset..offset + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .unwrap_or(0)
    }

    fn word32(&self, offset: usize) -> u32 {
        self.data
            .get(offset..offset + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    }
}

impl<'a> FromCV<'a> for FileSpaceHeaderRedo<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        if v[0].size < Self::MIN_LEN as u64 {
            return Err(ConvertError::Validity {
                reason: format!(
                    "ktfbhredo payload is too short for an opcode and save state: {} bytes",
                    v[0].size
                ),
            });
        }
        // Keep the 4-byte alignment padding: a 2-byte payload has no `instance`
        // field, but Oracle still prints the two padding bytes as the instance
        // (e.g. `Instance: 64` for a vector whose padding is `40 00`).
        Ok(Self { data: &v[0].data })
    }
}

// -----------------------------------------------------------------------------

/// Operation of `ktfbbredo` (OP:22.5), first payload byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileBitmapBlockOp {
    /// 1: printed as `Free Bits: `.
    FreeBits,
    /// 2: printed as `Use Bits: `.
    UseBits,
    /// 3: printed as `Copy Bits: `.
    CopyBits,
    /// Anything else, including 0: printed as `Unknown OPcode: `.
    Unknown(u8),
}

impl FileBitmapBlockOp {
    fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::FreeBits,
            2 => Self::UseBits,
            3 => Self::CopyBits,
            other => Self::Unknown(other),
        }
    }
}

/// The 12-byte layout of a `ktfbbredo` payload.
///
/// Same shape as [`FileSpaceHeaderRedoHead`]: the second byte has no effect on
/// the output. `Copy Bits` carries more data after `length`.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct FileBitmapBlockRedoHead {
    /// [`FileBitmapBlockOp`] as a raw byte.
    pub op: u8,
    /// No effect on the output.
    pub save: u8,
    pub instance: u16,
    pub begin: u32,
    pub length: u32,
}

/// `ktfbbredo - File BitMap Block Redo:` (OP:22.5).
#[derive(Debug)]
pub struct FileBitmapBlockRedo<'a> {
    data: &'a [u8],
}

impl FileBitmapBlockRedo<'_> {
    const LEN: usize = size_of::<FileBitmapBlockRedoHead>();

    pub fn op(&self) -> FileBitmapBlockOp {
        FileBitmapBlockOp::from_raw(self.data[0])
    }

    pub fn instance(&self) -> u16 {
        self.word16(2)
    }

    pub fn begin(&self) -> u32 {
        self.word32(4)
    }

    pub fn length(&self) -> u32 {
        self.word32(8)
    }

    fn word16(&self, offset: usize) -> u16 {
        self.data
            .get(offset..offset + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .unwrap_or(0)
    }

    fn word32(&self, offset: usize) -> u32 {
        self.data
            .get(offset..offset + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .unwrap_or(0)
    }
}

impl<'a> FromCV<'a> for FileBitmapBlockRedo<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        if v[0].size < Self::LEN as u64 {
            return Err(ConvertError::Validity {
                reason: format!(
                    "ktfbbredo payload is too short for a save area: {} bytes",
                    v[0].size
                ),
            });
        }
        Ok(Self {
            data: &v[0].data[..Self::LEN],
        })
    }
}
