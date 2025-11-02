use std::ops::Range;

use thiserror::Error;
use zerocopy::KnownLayout;

use crate::oracle::redo::Change;

#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("the conversion source was improperly aligned.")]
    Alignment { got: usize },
    #[error("the conversion source was of incorrect size {got}.")]
    Size { got: usize },
    #[error("the conversion source contained invalid data: {reason}.")]
    Validity { reason: String },
}

impl<S, D: KnownLayout + ?Sized> From<zerocopy::CastError<&[S], D>> for ConvertError {
    fn from(value: zerocopy::CastError<&[S], D>) -> Self {
        match value {
            zerocopy::ConvertError::Alignment(e) => ConvertError::Alignment {
                got: e.into_src().len(),
            },
            zerocopy::ConvertError::Size(e) => ConvertError::Size {
                got: e.into_src().len(),
            },
            zerocopy::ConvertError::Validity(_) => ConvertError::Validity {
                reason: "Infallible".to_string(),
            },
        }
    }
}

pub trait FromCV<'a>
where
    Self: Sized,
{
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError>;
}

#[derive(Debug)]
pub struct ChangeAndVector {
    pub change: Change,
    pub vector: Vec<VectorData>,
}

#[derive(Debug)]
pub struct VectorData {
    /// Vector data which has padding.
    pub data: Vec<u8>,

    /// Actual size of vector data (padding exclusive.)
    pub size: u64,

    /// Offsets of data from original redo file.
    pub ranges: Vec<Range<u64>>,
}
