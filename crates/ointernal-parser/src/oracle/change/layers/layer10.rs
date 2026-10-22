use bitfield_struct::bitfield;
use zerocopy::{FromBytes, Immutable, KnownLayout};

use crate::oracle::change::{ConvertError, VectorData, cv::FromCV, layers::layer11::RowOperation};

/// `kdxlin` / `kdxlde` redo.
///
/// `vector#0` is `KtbRedo` (identical to layer 11's `RowOperation`), `vector#1`
/// is the index metadata, and the remaining vectors carry the inserted keys.
#[derive(Debug)]
pub struct KdxRedo<'a> {
    pub op: &'a RowOperation,
    pub meta: KdxMetaWrapper<'a>,
}

impl<'a> FromCV<'a> for KdxRedo<'a> {
    fn from_vectors(v: &'a [VectorData]) -> Result<Self, ConvertError> {
        Ok(Self {
            op: FromBytes::ref_from_bytes(&v[0].data)?,
            meta: KdxMetaWrapper::parse(&v[1].data[..v[1].size as usize], &v[2..])?,
        })
    }
}

#[bitfield(u8)]
#[derive(KnownLayout, Immutable, FromBytes)]
pub struct KdxRedoFlags {
    #[bits(5)]
    unknown_1: u8,

    /// Multiple keys are inserted/deleted by this change.
    /// `#{AMND}`: clear this bit (`0x20` -> `0x00`) and Oracle prints `SINGLE`.
    #[bits(1)]
    pub array: bool,

    #[bits(2)]
    unknown_2: u8,
}

/// `vector#1` for a single-key change (6 bytes).
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct KdxMetaSingle {
    pub itl: u8,
    pub flags: KdxRedoFlags,
    pub sno: u16,
    pub row_size: u16,
}

/// `vector#1` for a multi-key (ARRAY) change (12 bytes + `num_keys` slots).
#[derive(KnownLayout, Immutable, FromBytes, Debug)]
#[repr(C)]
pub struct KdxMetaArray {
    pub itl: u8,
    pub flags: KdxRedoFlags,
    /// Slot number of the first key.
    pub sno: u16,
    pub row_size: u16,
    _unknown_1: u16,
    pub num_keys: u32,
    pub slots: [u16],
}

#[derive(Debug)]
pub enum KdxMetaRef<'a> {
    Single(&'a KdxMetaSingle),
    Array(&'a KdxMetaArray),
}

#[derive(Debug)]
pub struct KdxMetaWrapper<'a> {
    pub meta: KdxMetaRef<'a>,
    /// Insert keys; empty for `kdxlde` (delete leaf row), which addresses the
    /// rows by slot only.
    pub keys: Vec<&'a [u8]>,
    /// Per-key sizes (`vector#3`); empty unless `keys` came from an ARRAY
    /// change.
    pub key_sizes: Vec<u16>,
}

impl<'a> KdxMetaWrapper<'a> {
    fn parse(data: &'a [u8], key_vectors: &'a [VectorData]) -> Result<Self, ConvertError> {
        // Both shapes start with the 6-byte single header.
        let head: &KdxMetaSingle = FromBytes::ref_from_bytes(&data[..6])?;

        if head.flags.array() {
            let meta: &KdxMetaArray = FromBytes::ref_from_bytes(data)?;

            // `kdxlde` deletes by slot, so an ARRAY delete carries no key
            // vectors at all.
            let (key_data, sizes) = match key_vectors {
                [k, sz, ..] => (
                    &k.data[..k.size as usize],
                    <[u16]>::ref_from_bytes(&sz.data[..sz.size as usize])?,
                ),
                [] => (&[][..], &[][..]),
                _ => {
                    return Err(ConvertError::Validity {
                        reason: format!(
                            "kdx array change needs key and size vectors, got {}",
                            key_vectors.len()
                        ),
                    });
                }
            };

            let mut offset = 0usize;
            let keys = sizes
                .iter()
                .map(|size| {
                    let size = *size as usize;
                    let key = &key_data[offset..offset + size];
                    offset += size;
                    key
                })
                .collect();

            Ok(Self {
                meta: KdxMetaRef::Array(meta),
                keys,
                key_sizes: sizes.to_vec(),
            })
        } else {
            let keys = match key_vectors.first() {
                Some(v) => vec![&v.data[..v.size as usize]],
                None => Vec::new(),
            };

            Ok(Self {
                meta: KdxMetaRef::Single(head),
                keys,
                key_sizes: Vec::new(),
            })
        }
    }
}
