use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::oracle::{
    change::{
        ChangeAndVector,
        cv::{ConvertError, FromCV},
    },
    scn::Scn,
};

#[derive(KnownLayout, Immutable, FromBytes, IntoBytes, Debug, PartialEq, Eq)]
pub struct ObjInfo {
    pub obj: u32,
    pub type_: u16,
    pub itls: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BlockFormat<'a> {
    pub obj_info: &'a ObjInfo,
    pub scn: &'a Scn,
}

impl<'a> FromCV<'a> for BlockFormat<'a> {
    fn from_cv(cv: &'a ChangeAndVector) -> Result<Self, ConvertError> {
        Ok(Self {
            obj_info: FromBytes::ref_from_bytes(&cv.vector[0])?,
            scn: FromBytes::ref_from_bytes(&cv.vector[1])?,
        })
    }
}
