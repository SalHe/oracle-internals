use std::mem::transmute;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[derive(KnownLayout, Immutable, FromBytes, IntoBytes, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Scn {
    pub base: u32,
    pub wrapper: u16,
    pub marker: u16,
}

impl Scn {
    pub fn full_scn(self) -> u64 {
        let mut wrapper = self.wrapper as u32;
        if self.marker & 0xFFFE == 0xFFFE {
            if wrapper == 0xFFFF {
                return (self.base as u64) | 0xFFFF_FFFF_0000_0000u64;
            }

            if unsafe { transmute::<Scn, u64>(self) } == 0xFFFFFFFEFFFFFFFFu64 {
                return 0x7FFEFFFFFFFFFFFFu64;
            }
        }

        if self.wrapper & 0x8000 != 0 {
            wrapper = self.marker as u32 | ((wrapper & 0x7FFF) << 16);
        }

        (self.base as u64) | ((wrapper as u64) << 32)
    }
}

#[cfg(test)]
mod test {
    use zerocopy::FromBytes;

    use crate::oracle::scn::Scn;

    #[test]
    fn scn_ffff_ffff_ffff_ffff() {
        assert_eq!(
            0xFFFF_FFFF_FFFF_FFFF,
            Scn::read_from_bytes(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
                .unwrap()
                .full_scn()
        );
    }

    #[test]
    fn normal_scn() {
        assert_eq!(
            0x0000000000000001,
            Scn::read_from_bytes(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
                .unwrap()
                .full_scn()
        );
        assert_eq!(
            0x000000000027b373,
            Scn::read_from_bytes(&[0x73, 0xB3, 0x27, 0x00, 0x00, 0x80, 0x00, 0x00,])
                .unwrap()
                .full_scn()
        );
        assert_eq!(
            0x00000000004c3351,
            Scn::read_from_bytes(&[0x51, 0x33, 0x4C, 0x00, 0x00, 0x80, 0x00, 0x00,])
                .unwrap()
                .full_scn()
        );
    }

    #[test]
    fn marker_scn() {
        assert_eq!(
            0x0000fffe004c3351,
            Scn::read_from_bytes(&[0x51, 0x33, 0x4C, 0x00, 0x00, 0x80, 0xFE, 0xFF,])
                .unwrap()
                .full_scn()
        );
        assert_eq!(
            0xffffffff004c3351,
            Scn::read_from_bytes(&[0x51, 0x33, 0x4C, 0x00, 0xFF, 0xFF, 0xFE, 0xFF,])
                .unwrap()
                .full_scn()
        );
        assert_eq!(
            0x00000000004c3351,
            Scn::read_from_bytes(&[0x51, 0x33, 0x4C, 0x00, 0x00, 0x00, 0xFE, 0xFF,])
                .unwrap()
                .full_scn()
        );
        assert_eq!(
            0x7FFEFFFFFFFFFFFF,
            Scn::read_from_bytes(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF,])
                .unwrap()
                .full_scn()
        );
    }
}
