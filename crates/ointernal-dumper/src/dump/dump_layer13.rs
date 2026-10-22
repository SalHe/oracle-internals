use ointernal_parser::oracle::change::{
    SegmentManagement,
    layers::layer13::{
        self, BlockFormat, BlockLinkModify, FormatLobHashBucketBlock, FormatPagetableDatablock,
        FreelistModify,
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
