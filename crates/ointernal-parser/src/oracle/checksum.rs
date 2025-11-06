use zerocopy::{CastError, FromBytes};

use crate::oracle::redo::RedoBlockHead;

fn calc_checksum_impl(block: &[u8]) -> u16 {
    let mut checksum: u64 = 0;
    for i in 0..block.len() / 8 {
        let q = u64::read_from_bytes(&block[(i * 8)..((i + 1) * 8)]).unwrap();
        checksum ^= q;
    }

    checksum ^= checksum >> 32;
    checksum ^= (checksum) >> 16;
    (checksum & 0xFFFF) as u16
}

pub fn calc_checksum(block: &mut [u8]) -> Result<(u16, u16), CastError<&mut [u8], RedoBlockHead>> {
    let (head, _) = RedoBlockHead::mut_from_prefix(block).unwrap();

    // prepare magic number
    let original_magic = head.magic_number;
    if head.offset & 0x8000 != 0 {
        head.magic_number |= 1;
    }

    // reset checksum=0
    let original_checksum = head.checksum;
    head.checksum = 0;

    // calc
    let checksum = calc_checksum_impl(block);

    // recover block
    let (head, _) = RedoBlockHead::mut_from_prefix(block)?;

    head.checksum = original_checksum;
    head.magic_number = original_magic;

    Ok((checksum, original_checksum))
}

#[cfg(test)]
mod test {
    use crate::oracle::checksum::calc_checksum;

    #[test]
    fn checksum() {
        let mut block: [u8; _] = [
            1, 34, 0, 0, 1, 0, 0, 0, 62, 0, 0, 0, 0, 128, 232, 55, 0, 0, 0, 0, 0, 0, 0, 21, 3, 69,
            116, 112, 79, 82, 67, 76, 73, 78, 84, 69, 200, 19, 0, 0, 0, 64, 6, 0, 0, 2, 0, 0, 2, 0,
            2, 0, 3, 76, 116, 112, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 84, 32, 48, 48, 48, 49, 44, 32, 83, 32,
            48, 48, 48, 48, 48, 48, 48, 48, 54, 50, 44, 32, 83, 67, 78, 32, 48, 120, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 52, 99, 51, 51, 53, 49, 45, 48, 120, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 52, 99, 51, 51, 53, 55, 0, 4, 0, 0, 0, 67, 47, 87, 72, 115, 179,
            39, 0, 0, 128, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 81, 51, 76, 0, 0, 128, 0, 0, 0, 164, 88,
            72, 87, 51, 76, 0, 0, 128, 0, 0, 0, 164, 88, 72, 0, 0, 8, 18, 115, 179, 39, 0, 0, 128,
            0, 0, 67, 47, 87, 72, 81, 51, 76, 0, 0, 128, 0, 0, 0, 164, 88, 72, 0, 0, 0, 0, 17, 0,
            128, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 2, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 154, 185, 80, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 152, 231, 155, 30, 237, 9, 108, 152, 102,
            138, 180, 24, 22, 14, 56, 203, 144, 171, 138, 155, 24, 213, 79, 240, 191, 203, 242,
            107, 70, 62, 46, 150, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        // TODO make sure alignment of 8
        println!("{:?}", &block[0] as *const u8);
        let expected = 0x37e8u16;
        let (calc, recorded) = calc_checksum(&mut block).unwrap();
        assert_eq!(expected, calc);
        assert_eq!(expected, recorded);
        assert_eq!(calc, recorded);
    }
}
