use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::change::{ConvertError, VectorData, cv::FromCV};

/// Entry of the `KQR INFO:` section (OP:17.28), 8 bytes.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct KqrInfo {
    pub id: u32,
    /// Printed as the third number.
    pub value: u16,
    /// Printed as the second number.
    pub kind: u16,
}

/// Entry of the `KGL INFO:` section (OP:17.28), 20 bytes.
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct KglInfo {
    pub id: [u32; 4],
    /// Only the low byte is the count Oracle prints; the upper bytes are
    /// `#{AMND}` (patched to non-zero values without any output change).
    pub count: u32,
}

/// `STANDBY METADATA CACHE INVALIDATION` (OP:17.28).
///
/// Used for standby metadata cache invalidation after a commit on a system
/// table. Oracle prints both sections even when they are empty:
///
/// ```text
/// STANDBY METADATA CACHE INVALIDATION
/// KQR INFO:
/// [2c017bc3, 2, 0]
/// KGL INFO:
/// [504592d3.17ca2979.493abc5e.ef89d5f0, 1]
/// ```
#[derive(Debug)]
pub struct StandbyMetadataCacheInvalidation<'a> {
    /// `KQR INFO:` entries, vector 0.
    pub kqr: &'a [KqrInfo],
    /// `KGL INFO:` entries, vector 1.
    pub kgl: &'a [KglInfo],
}

impl<'a> FromCV<'a> for StandbyMetadataCacheInvalidation<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        let kqr = match v.first() {
            // An empty vector has no alignment, so `ref_from_bytes` cannot be used.
            Some(x) if x.size > 0 => <[KqrInfo]>::ref_from_bytes(&x.data[..x.size as usize])?,
            _ => &[],
        };
        let kgl = match v.get(1) {
            Some(x) if x.size > 0 => <[KglInfo]>::ref_from_bytes(&x.data[..x.size as usize])?,
            _ => &[],
        };
        Ok(Self { kqr, kgl })
    }
}
