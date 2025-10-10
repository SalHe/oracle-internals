use thiserror::Error;
use zerocopy::KnownLayout;

use crate::oracle::redo::Change;

#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("the conversion source was improperly aligned.")]
    Alignment { got: usize },
    #[error("the conversion source was of incorrect size {got}.")]
    Size { got: usize },
    #[error("the conversion source contained invalid data.")]
    Validity,
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
            zerocopy::ConvertError::Validity(_) => ConvertError::Validity,
        }
    }
}

pub trait FromCV<'a>
where
    Self: Sized,
{
    fn from_cv(cv: &'a ChangeAndVector) -> Result<Self, ConvertError>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChangeAndVector {
    pub change: Change,
    pub vector: Vec<Vec<u8>>,
}
