use ointernal_parser::oracle::change::{
    SegmentManagement,
    layers::layer13::{
        self, BlockFormat, BlockLinkModify, FormatLobHashBucketBlock, FormatPagetableDatablock,
        FreelistModify, Hwm, UpdateLhwmHhwm, UpdateLhwmHhwmOp,
    },
};

use crate::dump::Dump;

impl Dump for SegmentManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::BlockFormatRedo(Some(b)) => b.dump(),
            Self::BlockLinkModify(Some(b)) => b.dump(),
            Self::FreelistModify(Some(b)) => b.dump(),
            Self::FormatLobHashBucketBlock(Some(b)) => b.dump(),
            Self::FormatPagetableDatablock(Some(b)) => b.dump(),
            Self::UpdateLhwmHhwm(Some(b)) => b.dump(),
            _ => Ok(false),
        }
    }
}

impl Dump for BlockFormat<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "KTSFRBFMT (block format) redo: Segobjd: 0x{:08x} type: {} itls: {} cscn: 0x{:016x}",
            self.obj_info.obj,
            self.obj_info.type_,
            self.obj_info.itls,
            self.scn.full_scn()
        );
        Ok(true)
    }
}

impl Dump for BlockLinkModify<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!(
            "KTSFRBLNK (block link modify) redo:  Opcode: {}",
            match self.data.op_code() {
                Some(o) => match o {
                    layer13::BlockLinkModifyOp::LockClear => "LCLR (lock clear)",
                    layer13::BlockLinkModifyOp::LockSet => "LSET (lock set)",
                    layer13::BlockLinkModifyOp::LockWrite => "LWRT (lock write)",
                    layer13::BlockLinkModifyOp::ClearLock => "LCLK (clear lock)",
                },
                None => "???",
            }
        );
        println!("Next dba: {} itli: {}", self.data.dba, self.data.itli);
        Ok(true)
    }
}

impl Dump for FormatLobHashBucketBlock<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        let h = self.head;
        // Bits 0 and 1 select the bucket kind; any other combination is invalid.
        let kind = if h.flag & 1 != 0 {
            "CFS"
        } else if h.flag & 2 != 0 {
            "UFS"
        } else {
            "Invalid"
        };
        println!("KTSL - Format LOB Hash Bucket Block ({})", kind);
        if kind == "CFS" {
            println!(
                "objd: {} inc: {} inst:{} flag: 0x{:x} range: {}",
                h.objd,
                h.inc,
                h.inst,
                h.flag,
                match h.range {
                    0 => "2k-32k",
                    1 => "32k-64k",
                    2 => "64k-128k",
                    3 => "128k-256k",
                    4 => "256k-512k",
                    5 => "512k-1m",
                    6 => "1m-64m",
                    _ => "Invalid",
                }
            );
        } else {
            // Oracle ends this line with two spaces (the CFS-only range field is
            // skipped but its separator remains).
            println!(
                "objd: {} inc: {} inst:{} flag: 0x{:x}  ",
                h.objd, h.inc, h.inst, h.flag
            );
        }
        Ok(true)
    }
}

impl Dump for FreelistModify<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        println!("KTSFRGRP (fgb/shdr modify freelist) redo:");
        for one in &self.modifies {
            match one.data()? {
                Some((op, data)) => {
                    println!(
                        " Opcode: {}",
                        match op {
                            layer13::FreelistModifyBaseOp::MoveHwm => "HWMMV (move hwm)",
                            layer13::FreelistModifyBaseOp::ListNew => "LNEW (list new)",
                            layer13::FreelistModifyBaseOp::LinkBlock => "LUPD_LBLK (link block)",
                            layer13::FreelistModifyBaseOp::UnlinkBlock =>
                                "LUPD_UNLBLK (unlink block)",
                            layer13::FreelistModifyBaseOp::LinkList => "LUPD_LLIST (link a list)",
                            layer13::FreelistModifyBaseOp::UnlinkList =>
                                "LUPD_UNLIST (unlink a list)",
                            layer13::FreelistModifyBaseOp::SegListsCopy =>
                                "GSET_SEGLIST (seg lists copy)",
                            layer13::FreelistModifyBaseOp::XNTListsCopy =>
                                "GSET_XNTLIST (xnt lists copy)",
                        }
                    );

                    match data {
                        layer13::FreelistModifyDataBody::MoveHwm { nbk } => {
                            println!("NBK: {nbk}")
                        }
                        layer13::FreelistModifyDataBody::Other(data) => {
                            println!("Slot no: {}, Count: {}", data.slot_no, data.slot_count);
                            println!(
                                "Flag: = {} xid or slot0 ccnt:  0x{:04x}.{:03x}.{:08x} Head:  0x{:08x} Tail:  0x{:08x} ",
                                data.flag,
                                data.xid.usn,
                                data.xid.slot,
                                data.xid.sequence,
                                data.head,
                                data.tail,
                            );
                        }
                    }
                }
                None => println!(" Opcode: ???"),
            }
        }
        Ok(true)
    }
}

impl Dump for FormatPagetableDatablock<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        let h = self.head;
        // Oracle keeps the trailing space of the banner.
        println!("ktspbfredo - Format Pagetable Datablock ");
        println!(
            "Parent(l1) DBA:  0x{:08x} typ: {} objd: {} itls: {} fmt_flag: {} poff: {}",
            h.parent_dba,
            h.typ & 0x7fff,
            h.objd,
            h.itls as u64,
            h.fmt_flag,
            h.poff,
        );
        // Bit 15 of `typ` gates the second line.
        if h.typ & 0x8000 != 0 {
            println!(
                " cscn: 0x{:016x} inc#:{}",
                self.tail.cscn.full_scn(),
                self.tail.inc,
            );
        }
        Ok(true)
    }
}

impl Dump for UpdateLhwmHhwm<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        let d = self.data();
        match self.op() {
            UpdateLhwmHhwmOp::LastsInHeader => {
                println!("Lasts in Header");
                println!(
                    "LF: {} LS: {} LT: {} FT: {}",
                    d.high.ext, d.high.blk, d.high.ext_size, d.high.dba,
                );
            }
            UpdateLhwmHhwmOp::HighHwm => {
                println!("High HWM");
                dump_hwm(&d.high, d.high_lfdba, true);
            }
            UpdateLhwmHhwmOp::LowHwm => {
                println!("Low HWM");
                dump_hwm(&d.low, d.low_lfdba, true);
            }
            UpdateLhwmHhwmOp::BothHwm => {
                println!("Both the HWMs");
                println!("Low HWM");
                dump_hwm(&d.low, d.low_lfdba, false);
                println!("High HWM");
                dump_hwm(&d.high, d.high_lfdba, true);
            }
            UpdateLhwmHhwmOp::HintDba => println!("hint dba:  0x{:08x}", d.high.ext),
            UpdateLhwmHhwmOp::ExpressAllocationArea => {
                println!("Update Express Allocation Area");
                println!(
                    "Allocation Area : {}",
                    if self.express_allocation_full() {
                        "FULL"
                    } else {
                        "EMPTY"
                    }
                );
            }
            // Every other selector value produces no output at all.
            UpdateLhwmHhwmOp::Unknown(_) => {}
        }
        Ok(true)
    }
}

fn dump_hwm(x: &Hwm, lfdba: u32, last: bool) {
    // Every value is left-justified in a 6-character field; the first two
    // fields are followed by an explicit separating space.
    println!(
        "      Highwater::  0x{:08x}  ext#: {:<6} blk#: {:<6} ext size: {:<6}",
        x.dba, x.ext, x.blk, x.ext_size,
    );
    println!("  #blocks in seg. hdr's freelists: {:<6}", x.freelists);
    println!("  #blocks below: {:<6}", x.below);
    println!("  mapblk  0x{:08x}  offset: {:<6}", x.mapblk, x.offset);
    if last {
        // The last section leaves the line open with one trailing space: a
        // following change header is glued to it, and the record separator
        // supplies the newline (and another space).
        print!("lfdba:  0x{:08x} ", lfdba);
    } else {
        println!("lfdba:  0x{:08x}", lfdba);
    }
}
