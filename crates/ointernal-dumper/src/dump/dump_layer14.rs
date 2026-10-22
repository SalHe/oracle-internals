use ointernal_parser::oracle::change::{
    ExtentManagement,
    layers::layer14::{ExtentMapOp, ExtentMapRedo, LockExtent},
};

use crate::dump::Dump;

impl Dump for ExtentManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::ClearExtentControlLock(_) => {
                println!("ktecush redo: clear extent control lock");
                Ok(true)
            }
            Self::LockExtent(Some(x)) => x.dump(),
            Self::ExtentMapRedo(Some(x)) => x.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for LockExtent<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "ktelk redo: xid:  0x{:04x}.{:03x}.{:08x}",
            self.xid.usn, self.xid.slot, self.xid.sequence,
        );
        Ok(true)
    }
}

impl Dump for ExtentMapRedo<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!("kteop redo - redo operation on extent map");
        let w = |i| self.word(i);
        match self.op() {
            ExtentMapOp::Add => {
                println!("   ADD: dba:0x{:x} len:{} at offset:{}", w(1), w(2), w(3));
            }
            ExtentMapOp::Delete => {
                // The trailing space is part of Oracle's format.
                println!("   DELETE: entry:{}", w(1));
                println!("   shift back: dba:0x{:x} len:{} ", w(2), w(3));
            }
            ExtentMapOp::LinkMap => {
                println!("   LINKMAP: nextmap:0x{:x}  clear at: {}", w(1), w(2));
            }
            ExtentMapOp::Unlink => {
                println!("   UNLINK: nextmap:0x{:x} extno:{}", w(1), w(2));
                for (dba, len) in list_pairs(self, w(2)) {
                    println!("      dba:0x{:x} len:{}", dba, len);
                }
            }
            ExtentMapOp::SetStat => {
                println!(
                    "   SETSTAT: exts:{} blks:{} lastmap:0x{:x} mapcnt:{}",
                    w(1),
                    w(2),
                    w(3),
                    w(4),
                );
            }
            ExtentMapOp::Resize => {
                // Oracle prints "%" literally here (a format-string bug).
                println!("   RESIZE: entry:{} delta:%", w(1));
            }
            ExtentMapOp::SetHwm => {
                println!(
                    "   SETHWM:       Highwater::  0x{:08x}  ext#: {:<7}blk#: {:<7}ext size: {:<6}",
                    w(4),
                    w(1),
                    w(2),
                    w(3),
                );
                println!("  #blocks in seg. hdr's freelists: {:<6}", w(7));
                println!("  #blocks below: {:<6}", w(8));
                println!("  mapblk  0x{:08x}  offset: {:<6}", w(5), w(6));
            }
            ExtentMapOp::UpdateExtentMap => {
                println!(
                    "   UPDXNT: extent:{} add:{}",
                    w(1),
                    if w(2) != 0 { "TRUE" } else { "FALSE" },
                );
            }
            ExtentMapOp::MoveMap => {
                println!("   MVMAP: extno:{}, entries:{}", w(2), w(1));
                for (dba, len) in list_pairs(self, w(1)) {
                    println!("      dba:0x{:x} len:{}", dba, len);
                }
            }
            ExtentMapOp::ChangeObj => {
                println!("   CDOBJ: new object number:{}", w(1));
            }
            ExtentMapOp::MoveAxt => {
                println!("  MOVAXT: offset:{} entries:{}", w(1), w(2));
                for i in 0..list_len(self, w(2), 2) {
                    println!(
                        "  [{}] fdba:0x{:08x} bdba:0x{:08x}",
                        i,
                        w(3 + 2 * i),
                        w(4 + 2 * i),
                    );
                }
            }
            ExtentMapOp::AddAxt => {
                // Oracle's format drops the leading 0 of "0x" before `fdba`.
                println!(
                    "  ADDAXT: offset:{} fdba:x{:08x} bdba:0x{:08x}",
                    w(1),
                    w(2),
                    w(3),
                );
            }
            ExtentMapOp::MoveRet => {
                println!("  MOVRET: offset:{} entries:{}", w(1), w(2));
                for i in 0..list_len(self, w(2), 1) {
                    println!("  [{}] ctime:{}", i, w(3 + i));
                }
            }
            ExtentMapOp::AddRet => {
                println!("  ADDRET: offset:{} ctime:{}", w(1), w(2));
            }
            ExtentMapOp::Other(_) => {
                println!("   DEFAULT: ???");
            }
        }
        Ok(true)
    }
}

/// Number of inline array entries that fit in the payload.
///
/// Oracle prints the number of entries stored in the operation word even when
/// that many are not logged, so a hand-modified file can make it read past the
/// payload. We stop at the end of the vector instead.
fn list_len(x: &ExtentMapRedo<'_>, wanted: u32, stride: usize) -> usize {
    let available = x.word_count().saturating_sub(3) / stride;
    (wanted as usize).min(available)
}

fn list_pairs<'a>(x: &'a ExtentMapRedo<'_>, wanted: u32) -> impl Iterator<Item = (u32, u32)> + 'a {
    let n = list_len(x, wanted, 2);
    (0..n).map(move |i| (x.word(3 + 2 * i), x.word(4 + 2 * i)))
}
