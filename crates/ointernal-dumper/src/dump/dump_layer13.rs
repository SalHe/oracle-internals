use ointernal_parser::oracle::change::{
    SegmentManagement,
    layers::layer13::{self, BlockFormat, BlockLinkModify, FreelistModify},
};

use crate::dump::DumpRedo;

impl DumpRedo for SegmentManagement<'_> {
    fn dump(&self) -> anyhow::Result<bool> {
        match self {
            Self::BlockFormatRedo(Some(b)) => b.dump(),
            Self::BlockLinkModify(Some(b)) => b.dump(),
            Self::FreelistModify(Some(b)) => b.dump(),
            _ => return Ok(false),
        }
    }
}

impl DumpRedo for BlockFormat<'_> {
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

impl DumpRedo for BlockLinkModify<'_> {
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
        println!(
            "Next dba: 0x{:08x} itli: {}",
            self.data.dba.0, self.data.itli
        );
        Ok(true)
    }
}

impl DumpRedo for FreelistModify<'_> {
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
