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
