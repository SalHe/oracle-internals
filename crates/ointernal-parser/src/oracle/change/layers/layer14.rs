use zerocopy::FromBytes;

use crate::oracle::change::{ConvertError, VectorData, cv::FromCV, layers::Xid};

// -----------------------------------------------------------------------------

/// `ktelk redo - lock extent (map)`: the locked transaction.
#[derive(Debug)]
pub struct LockExtent<'a> {
    pub xid: &'a Xid,
}

impl<'a> FromCV<'a> for LockExtent<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            xid: FromBytes::ref_from_bytes(&v[0].data[..v[0].size as usize])?,
        })
    }
}

// `ktecush redo: clear extent control lock` (OP:14.1) carries no vector data,
// so it is registered without a payload type.

// -----------------------------------------------------------------------------

/// Operation of a `kteop redo - redo operation on extent map` payload (OP:14.4).
///
/// The names come from Oracle's own dump lines, which print the raw operation
/// name in upper case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtentMapOp {
    /// `ADD`
    Add,
    /// `DELETE`
    Delete,
    /// `LINKMAP`
    LinkMap,
    /// `UNLINK`
    Unlink,
    /// `SETSTAT`
    SetStat,
    /// `RESIZE`
    Resize,
    /// `SETHWM`
    SetHwm,
    /// `UPDXNT`
    UpdateExtentMap,
    /// `MVMAP`
    MoveMap,
    /// `CDOBJ`
    ChangeObj,
    /// `MOVAXT`
    MoveAxt,
    /// `ADDAXT`
    AddAxt,
    /// `MOVRET`
    MoveRet,
    /// `ADDRET`
    AddRet,
    /// Anything else, printed as `DEFAULT: ???` by Oracle.
    Other(u32),
}

impl ExtentMapOp {
    fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Add,
            2 => Self::Delete,
            3 => Self::LinkMap,
            4 => Self::Unlink,
            5 => Self::SetStat,
            6 => Self::Resize,
            7 => Self::SetHwm,
            8 => Self::UpdateExtentMap,
            9 => Self::MoveMap,
            10 => Self::ChangeObj,
            11 => Self::MoveAxt,
            12 => Self::AddAxt,
            13 => Self::MoveRet,
            14 => Self::AddRet,
            other => Self::Other(other),
        }
    }
}

/// `kteop redo - redo operation on extent map` (OP:14.4).
///
/// The payload starts with a fixed 9-word (36-byte) header. Only the first
/// words are meaningful for each [`ExtentMapOp`]; the list operations
/// (`Unlink`, `MoveMap`, `MoveAxt`, `MoveRet`) append an array of pairs or
/// values after the third word. Because the payload length depends on the
/// operation, the words are read on demand instead of being mapped onto a
/// fixed `#[repr(C)]` struct.
#[derive(Debug)]
pub struct ExtentMapRedo<'a> {
    /// The whole vector payload, trailing alignment padding excluded.
    pub data: &'a [u8],
}

impl ExtentMapRedo<'_> {
    /// Raw little-endian 32-bit word `i`; words past the payload read as 0.
    pub fn word(&self, i: usize) -> u32 {
        self.data
            .get(i * 4..i * 4 + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("slice of 4 bytes")))
            .unwrap_or(0)
    }

    /// Number of whole words in the payload.
    pub fn word_count(&self) -> usize {
        self.data.len() / 4
    }

    /// The operation stored in the low byte of the first word. Oracle keeps
    /// the same low byte when this payload is embedded in an undo record
    /// (where the first word is seen as `0x7f07`, still `SETHWM`).
    pub fn op(&self) -> ExtentMapOp {
        ExtentMapOp::from_raw(self.word(0) & 0xff)
    }
}

impl<'a> FromCV<'a> for ExtentMapRedo<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let data = &v[0].data[..v[0].size as usize];
        if data.len() < 4 {
            return Err(ConvertError::Validity {
                reason: format!(
                    "kteop redo payload is too short for an operation code: {} bytes",
                    data.len(),
                ),
            });
        }
        Ok(Self { data })
    }
}
