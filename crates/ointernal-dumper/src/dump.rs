use std::path::PathBuf;

use colored::Colorize;
use num_traits::FromPrimitive;
use ointernal_parser::oracle::change::{self, ChangeAndVector, Op};
use ointernal_parser::oracle::reader::ReaderError;
use ointernal_parser::oracle::redo::MiscFlags;
use ointernal_parser::oracle::timestamp::DateTime as OracleDateTime;
use ointernal_parser::oracle::{
    file::FileHeaderBlock,
    reader::{RBA, Record, RedoFileReader},
    redo::{RedoFileHead, RedoRecordHeadCommon},
    timestamp::OracleTimestamp,
};

fn print_example_infos() {
    // some examples?
    println!(" Container ID: 0");
    println!(" Container UID: 0");
    println!(" Opcodes *.*");
    println!(" Container ID: 0");
    println!(" Container UID: 0");
    println!(" RBAs: 0x000000.00000000.0000 thru 0xffffffff.ffffffff.ffff");
    println!(" SCNs: scn: 0x0000000000000000 thru scn: 0xffffffffffffffff");
    println!(" Times: creation thru eternity");
}

fn print_header_info(fh: &FileHeaderBlock, rh: &RedoFileHead) {
    println!(" FILE HEADER:");
    println!("	Compatibility Vsn = {}=0x{:08X}", rh.vsn, rh.vsn);
    println!(
        "	Db ID={}=0x{:08X}, Db Name='{}'",
        rh.db_id,
        rh.db_id,
        String::from_utf8_lossy(&rh.db_name),
    );
    println!(
        "	Activation ID={}=0x{:08x}",
        rh.activation_id, rh.activation_id,
    );
    println!(
        "	Control Seq={}=0x{:04x}, File size={}=0x{:x}",
        rh.control_seq, rh.control_seq, rh.file_size, rh.file_size,
    );
    println!(
        "	File Number={}, Blksiz={}, File Type={} {}",
        rh.file_number,
        fh.header.block_size,
        rh.file_type,
        match rh.file_type {
            2 => "LOG",
            _ => "???",
        }
    );
    println!(r#" descrip:"{}""#, String::from_utf8_lossy(&rh.description));
    println!(
        " thread: {} nab: 0x{:x} seq: 0x{:08x} hws: 0x{:x} eot: {} dis: {}",
        rh.thread, rh.nab, rh.head.seq, rh.hws, rh.dis, rh.eot,
    );
    println!(
        " resetlogs count: 0x{:08x} scn: 0x{:016x}",
        rh.reset_logs_count,
        rh.reset_log_scn.full_scn(),
    );
    println!(
        " prev resetlogs count: 0x{:08x} scn: 0x{:016x}",
        rh.prev_reset_logs_count,
        rh.prev_reset_logs_scn.full_scn(),
    );
    println!(
        " Low  scn: 0x{:016x} {}",
        rh.low_scn.full_scn(),
        OracleDateTime::from(rh.low_scn_timestamp)
    );
    println!(
        " Next scn: 0x{:016x} {}",
        rh.next_scn.full_scn(),
        OracleDateTime::from(rh.next_scn_timestamp),
    );
    println!(
        " Enabled scn: 0x{:016x} {}",
        rh.enabled_scn.full_scn(),
        OracleDateTime::from(rh.enabled_scn_timestamp),
    );
    println!(
        " Thread closed scn: 0x{:016x} {}",
        rh.thread_closed_scn.full_scn(),
        OracleDateTime::from(rh.thread_closed_scn_timestamp),
    );
    println!(" Real next scn: 0x{:016x}", rh.real_next_scn.full_scn());
    println!(" Disk cksum: 0x{:04x} Calc cksum: 0x????", rh.head.checksum); // TODO calc checksum
    println!(
        " Terminal recovery stop scn: 0x{:016x}",
        rh.terminal_recovery_stop_scn.full_scn()
    );
    println!(
        " Terminal recovery  {}",
        OracleDateTime::from(rh.terminal_recovery_timestamp)
    );
    println!(
        " Most recent redo scn: 0x{:016x}",
        rh.most_recent_redo_scn.full_scn()
    );
    println!(" Largest LWN: {} blocks", rh.largest_lwn);

    let flags = MiscFlags::from_bits(rh.misc_flags).unwrap();
    println!(
        " End-of-redo stream : {}",
        if flags.contains(MiscFlags::END_OF_STREAM) {
            "Yes"
        } else {
            "No"
        }
    );
    println!(" {}", flags.protected_mode());
    println!(" Miscellaneous flags: 0x{:08x}", rh.misc_flags);
    println!(" Miscellaneous second flags: 0x{:x}", rh.misc_flags2);
    println!(
        " Thread internal enable indicator: thr: {}, seq: {} scn: 0x{:016x}",
        rh.thread_internal_enable_tid,
        rh.thread_internal_enable_seq,
        rh.thread_internal_enable_scn.base,
    );

    if rh.standby_apply_delay_minutes != 0 {
        println!(
            " Standby Apply Delay: {} minute(s) ",
            rh.standby_apply_delay_minutes
        );
    }

    if rh.standby_log_close_time.0 != 0 {
        println!(
            " Standby Log Close Time:  {}",
            OracleDateTime::from(rh.standby_log_close_time)
        );
    }

    println!(" Zero blocks: {}", rh.zero_blocks);
    println!(" Format ID is {}", rh.format_id);
    println!(" redo log key is {}", hex::encode(rh.log_key));
    println!(" redo log key flag is {:x}(HEX)", rh.redo_key_flag);
    println!(
        " DAX High watermark block number: {}",
        rh.dax_hwm_block_number
    );
    println!(" DAX flags: 0x{:04x}", rh.dax_flags);
    println!(" DAX checksum: 0x{:04x}", rh.dax_checksum);

    print!(" Enabled redo threads: ");
    for i in 0..rh.threads_count() {
        if rh.is_thread_enabled(i) {
            print!("{} ", i);
        }
    }
    println!();
}

fn print_record_head(i: usize, rba: RBA, head: &RedoRecordHeadCommon, ts: OracleTimestamp) {
    println!(
        "{} - Thread: 1 RBA: 0x{:06x}.{:08x}.{:04x} LEN: 0x{:04x} VLD: 0x{:02x} CON_UID: {}",
        format!("REDO RECORD#{i}").bold().green(),
        rba.seq,
        rba.block_id,
        rba.offset,
        head.len,
        head.vld,
        head.con_uid,
    );
    println!(
        "SCN: 0x{:016x} SUBSCN:  {} {}",
        head.scn().full_scn(),
        head.sub_scn,
        OracleDateTime::from(ts),
    );
    println!(
        "(LWN RBA: 0x{:06x}.{:08x}.{:04x} LEN: 0x???????? NST: 0x???? SCN: 0x????????????????)",
        rba.seq, rba.block_id, rba.offset,
    );
}

fn print_segment_management(x: change::SegmentManagement) -> anyhow::Result<bool> {
    use change::SegmentManagement as X;
    match x {
        X::BlockFormatRedo(Some(b)) => {
            println!(
                "KTSFRBFMT (block format) redo: Segobjd: 0x{:08x} type: {} itls: {} cscn: 0x{:016x}",
                b.obj_info.obj,
                b.obj_info.type_,
                b.obj_info.itls,
                b.scn.full_scn()
            );
            Ok(true)
        }
        X::BlockLinkModify(Some(b)) => {
            println!(
                "KTSFRBLNK (block link modify) redo:  Opcode: {}",
                match b.data.op_code() {
                    Some(o) => match o {
                        change::layers::layer13::BlockLinkModifyOp::LockClear =>
                            "LCLR (lock clear)",
                        change::layers::layer13::BlockLinkModifyOp::LockSet => "LSET (lock set)",
                        change::layers::layer13::BlockLinkModifyOp::LockWrite =>
                            "LWRT (lock write)",
                        change::layers::layer13::BlockLinkModifyOp::ClearLock =>
                            "LCLK (clear lock)",
                    },
                    None => "???",
                }
            );
            println!("Next dba: 0x{:08x} itli: {}", b.data.dba.0, b.data.itli);
            Ok(true)
        }
        X::FreelistModify(Some(b)) => {
            println!("KTSFRGRP (fgb/shdr modify freelist) redo:");
            for one in b.modifies {
                match one.data()? {
                    Some((op, data)) => {
                        println!(
                            " Opcode: {}",
                            match op {
                                change::layers::layer13::FreelistModifyBaseOp::MoveHwm =>
                                    "HWMMV (move hwm)",
                                change::layers::layer13::FreelistModifyBaseOp::ListNew =>
                                    "LNEW (list new)",
                                change::layers::layer13::FreelistModifyBaseOp::LinkBlock =>
                                    "LUPD_LBLK (link block)",
                                change::layers::layer13::FreelistModifyBaseOp::UnlinkBlock =>
                                    "LUPD_UNLBLK (unlink block)",
                                change::layers::layer13::FreelistModifyBaseOp::LinkList =>
                                    "LUPD_LLIST (link a list)",
                                change::layers::layer13::FreelistModifyBaseOp::UnlinkList =>
                                    "LUPD_UNLIST (unlink a list)",
                                change::layers::layer13::FreelistModifyBaseOp::SegListsCopy =>
                                    "GSET_SEGLIST (seg lists copy)",
                                change::layers::layer13::FreelistModifyBaseOp::XNTListsCopy =>
                                    "GSET_XNTLIST (xnt lists copy)",
                            }
                        );

                        match data {
                            change::layers::layer13::FreelistModifyDataBody::MoveHwm { nbk } => {
                                println!("NBK: {nbk}")
                            }
                            change::layers::layer13::FreelistModifyDataBody::Other(data) => {
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
        _ => return Ok(false),
    }
}

fn print_tx_undo_management(x: change::TransactionUndoManagement) -> anyhow::Result<bool> {
    use change::TransactionUndoManagement as X;
    match x {
        X::UndoHeaderRedo(Some(b)) => {
            println!(
                "ktudh redo: slt: 0x{:04x} sqn: 0x{:08x} flg: 0x{:04x} siz: {} fbi: {}",
                b.data.slt, b.data.sqn, b.data.flag, b.data.size, b.data.fbi,
            );
            println!(
                "            uba: 0x{:08x}.{:04x}.{:02x}    pxid:  0x{:04x}.{:03x}.{:08x}        pdbuid:{}",
                b.data.uba.dba.0,
                b.data.uba.seq,
                b.data.uba.record,
                b.data.parent_xid.usn,
                b.data.parent_xid.slot,
                b.data.parent_xid.sequence,
                b.pdbuid,
            );
            Ok(true)
        }
        X::ChangeTxSlotStatus(Some(b)) => {
            print!(
                "ktucm redo: slt: 0x{:04x} sqn: 0x{:08x} srt: {} sta: {} flg: 0x{:x}",
                b.data.slot, b.data.seq, b.data.srt, b.data.sta, b.data.flags,
            );

            if let Some(d) = b.data2 {
                print!(
                    " ktucf redo: uba: 0x{:08x}.{:04x}.{:02x} ext: {} spc: {} fbi: {} ",
                    d.uba.dba.0, d.uba.seq, d.uba.seq, d.ext, d.spc, d.fbi
                );
            }

            if let Some(d) = b.scn {
                print!("\nscn:  0x{:016x}", d.full_scn());
            }

            println!();
            Ok(true)
        }
        _ => return Ok(false),
    }
}

fn print_change_table(x: change::Table) -> anyhow::Result<bool> {
    use change::Table as X;
    use change::layers::layer11;
    match x {
        X::InsertRowPiece(Some(irp)) => {
            println!("KTB Redo ");
            println!(
                "op: 0x{:02}  ver: 0x{:02}  ",
                irp.op.b1.into_bits(), // display byte1, not only op code's bitfields
                irp.op.b2.version(),
            );
            println!(
                "compat bit: {} ({}) padding: {}",
                layer11::IRPByte2::new()
                    .with_post_11(irp.op.b2.post_11())
                    .into_bits(),
                if irp.op.b2.post_11() {
                    "post-11"
                } else {
                    "pre-11"
                },
                if irp.op.b2.post_11() { 1 } else { 0 }
            );
            match irp.op.b1.op_ch() {
                Some(ch) => {
                    print!("op: {ch}  ");
                    let rest = irp.op.rest()?;
                    println!(
                        "xid:  0x{:04x}.{:03x}.{:08x}    uba: 0x{:08x}.{:04x}.{:02x}",
                        rest.xid.usn,
                        rest.xid.slot,
                        rest.xid.sequence,
                        rest.uba.dba.0,
                        rest.uba.seq,
                        rest.uba.record,
                    );
                }
                None => println!("op: [ktbdir] invalid op: 0x{:02X} ", irp.op.b1.into_bits()),
            };

            println!(
                "KDO Op code: {} row dependencies {}",
                match layer11::KdoOp::from_u8(irp.meta.byte1.op()) {
                    Some(op) => match op {
                        layer11::KdoOp::IUR => "IUR",
                        layer11::KdoOp::IRP => "IRP",
                        layer11::KdoOp::DRP => "DRP",
                        layer11::KdoOp::LKR => "LKR",
                        layer11::KdoOp::URP => "URP",
                        layer11::KdoOp::ORP => "ORP",
                        layer11::KdoOp::MFC => "MFC",
                        layer11::KdoOp::CFA => "CFA",
                        layer11::KdoOp::CKI => "CKI",
                        layer11::KdoOp::SKL => "SKL",
                        layer11::KdoOp::QMI => "QMI",
                        layer11::KdoOp::QMD => "QMD",
                        layer11::KdoOp::DSC => "DSC",
                        layer11::KdoOp::LMN => "LMN",
                        layer11::KdoOp::LLB => "LLB",
                        layer11::KdoOp::SHK => "SHK",
                        layer11::KdoOp::CMP => "CMP",
                        layer11::KdoOp::DCU => "DCU",
                        layer11::KdoOp::MRK => "MRK",
                        layer11::KdoOp::IRP2 => "IRP2",
                        layer11::KdoOp::DRP2 => "DRP2",
                        layer11::KdoOp::CMP2 => "CMP2",
                        layer11::KdoOp::DGD => "DGD",
                    }
                    .to_string(),
                    None => format!("{}", irp.meta.byte1.op()),
                },
                // TODO verify?
                if irp.meta.byte1.row_dependency() {
                    "Enabled"
                } else {
                    "Disabled"
                }
            );

            println!(
                "  xtype: {} flags: 0x{:08x}  bdba: 0x{:08x}  hdba: 0x{:08x}",
                match layer11::IRPXtype::from_u8(irp.meta.xtype.typ()) {
                    Some(xt) => match xt {
                        layer11::IRPXtype::XA => "XA",
                        layer11::IRPXtype::XR => "XR",
                        layer11::IRPXtype::CR => "CR",
                    }
                    .to_string(),
                    None => format!("{}", irp.meta.xtype.typ()),
                },
                irp.meta.xtype.with_typ(0).into_bits(), // clear type, flags remained
                irp.meta.bdba.0,
                irp.meta.hdba.0,
            );
            println!(
                "itli: {}  ispac: {}  maxfr: {}",
                irp.meta.itli, irp.meta.ispac, irp.meta.maxfr,
            );
            println!(
                "tabn: {} slot: {}(0x{:x}) size/delt: {}",
                irp.meta.tabn as i32, // seems dumped as i32 in Oracle
                irp.meta.slot,
                irp.meta.slot,
                irp.meta.delta_size,
            );
            println!(
                "fb: {}{}{}{}{}{}{}{} lb: 0x{:x}  cc: {}",
                if irp.meta.fb_flags.k() { 'K' } else { '-' },
                if irp.meta.fb_flags.c() { 'C' } else { '-' },
                if irp.meta.fb_flags.h() { 'H' } else { '-' },
                if irp.meta.fb_flags.d() { 'D' } else { '-' },
                if irp.meta.fb_flags.f() { 'F' } else { '-' },
                if irp.meta.fb_flags.l() { 'L' } else { '-' },
                if irp.meta.fb_flags.p() { 'P' } else { '-' },
                if irp.meta.fb_flags.n() { 'N' } else { '-' },
                irp.meta.lb,
                irp.meta.column_count,
            );

            print!("null: ");
            for c in 0..irp.meta.column_count {
                let n = match irp.meta.is_col_null(c) {
                    Some(is_null) => {
                        if is_null {
                            "N"
                        } else {
                            "-"
                        }
                    }
                    None => "?",
                };
                print!("{}", n);
            }
            println!();

            Ok(true)
        }
        _ => Ok(false),
    }
}

fn print_change(
    i: usize,
    force_print_vector: bool,
    change: &ChangeAndVector,
) -> anyhow::Result<()> {
    println!(
        "{} CON_ID:{} TYP:{} CLS:{} AFN:{} DBA:{:08x} OBJ:{} SCN:0x{:016x} SEQ:{} OP:{}.{} ENC:? RBL:? FLG:0x{:04x}",
        format!("CHANGE #{i} ").green(),
        change.change.cont_id,
        change.change.type_,
        change.change.block_class,
        change.change.file_number,
        change.change.dba.0,
        change.change.obj(),
        change.change.scn.full_scn(),
        change.change.seq,
        change.change.layer,
        change.change.op,
        change.change.flags,
    );

    let printed = match change.parse() {
        Ok(Op::SegmentManagement(x)) => print_segment_management(x)?,
        Ok(Op::TransactionUndoManagement(x)) => print_tx_undo_management(x)?,
        Ok(Op::Table(x)) => print_change_table(x)?,
        Err(e) => Err(e)?,
        _ => false,
    };

    if !printed || force_print_vector {
        for (i, v) in change.vector.iter().enumerate() {
            println!(
                " {} {}",
                format!("vector#{i}[{}]", v.len()).underline(),
                hex::encode(v)
            );
        }
    }

    Ok(())
}

macro_rules! eof_return_ok {
    ($e:expr) => {{
        let r = $e;
        if let Err(ReaderError::Eof) = r {
            println!("END OF REDO DUMP");
            return Ok(());
        }

        r
    }};
}

pub fn read_redo(file: PathBuf, force_print_vector: bool) -> anyhow::Result<()> {
    println!("DUMP OF REDO FROM FILE {file:?}");
    print_example_infos();

    let mut reader = RedoFileReader::from_file(file)?;
    reader.reload_header()?;

    let file_header = reader.file_header();
    let redo_file_header = reader.redo_file_header();
    print_header_info(file_header, redo_file_header);

    let mut i_record = 1usize;
    while let Some((rba, record)) = eof_return_ok!(reader.begin_read_record())? {
        println!();
        match record {
            Record::V1(v1) => print_record_head(i_record, rba, &v1.common, OracleTimestamp(0)),
            Record::V2(v2) => print_record_head(i_record, rba, &v2.common, v2.begin_time),
        }

        let mut i_change = 1usize;

        while let Some(change) = eof_return_ok!(reader.read_change())? {
            print_change(i_change, force_print_vector, &change)?;
            i_change += 1;
        }
        i_record += 1;
    }
    Ok(())
}
