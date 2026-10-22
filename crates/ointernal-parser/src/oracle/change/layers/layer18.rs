use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::change::{ConvertError, VectorData, cv::FromCV};

/// Reuse type stored in [`ReuseRedoEntryHead::type_`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReuseType {
    /// Object reuse (`Object reuse: pdb=.., tsn=.. objd=..`).
    Object,
    /// Range reuse.
    Range,
    /// Prewarm range reuse.
    PrewarmRange,
    /// Any other value (0, 3, 5, ...) is dumped as local range reuse.
    LocalRange(u32),
}

impl ReuseType {
    fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Object,
            2 => Self::Range,
            4 => Self::PrewarmRange,
            other => Self::LocalRange(other),
        }
    }
}

/// `Reuse redo entry` (OP:18.3), 24 bytes.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct ReuseRedoEntryHead {
    /// Raw reuse type, see [`ReuseType`].
    pub type_: u32,
    pub tsn: u32,
    /// Object id, used by [`ReuseType::Object`] only.
    pub objd: u32,
    /// Base DBA, used by the range reuse types only.
    pub base: u32,
    pub nblks: u32,
    pub pdb: u32,
}

#[derive(Debug)]
pub struct ReuseRedoEntry<'a> {
    pub head: &'a ReuseRedoEntryHead,
}

impl ReuseRedoEntry<'_> {
    pub fn reuse_type(&self) -> ReuseType {
        ReuseType::from_raw(self.head.type_)
    }
}

impl<'a> FromCV<'a> for ReuseRedoEntry<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            head: FromBytes::ref_from_bytes(&v[0].data[..v[0].size as usize])?,
        })
    }
}
