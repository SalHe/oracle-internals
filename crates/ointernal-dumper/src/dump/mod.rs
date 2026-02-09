mod dump_layer11;
mod dump_layer13;
mod dump_layer5;

use std::path::PathBuf;

use colored::Colorize;
use ointernal_parser::oracle::change::{ChangeAndVector, Op, OpDataError};
use ointernal_parser::oracle::reader::ReaderError;
use ointernal_parser::oracle::timestamp::DateTime as OracleDateTime;
use ointernal_parser::oracle::{
    file::FileHeaderBlock,
    reader::{RBA, Record, RedoFileReader},
    redo::{RedoFileHead, RedoRecordHeadCommon},
    timestamp::OracleTimestamp,
};

use crate::filter;
use crate::utils::Hex;

trait Dump {
    fn dump(&self) -> anyhow::Result<bool>;
}

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

fn print_header_info(fh: &FileHeaderBlock, rh: &RedoFileHead, checksum: u16) {
    println!(" FILE HEADER:");
    println!("	Compatibility Vsn = {}=0x{:08X}", rh.vsn, rh.vsn);
    println!(
        "	Db ID={}=0x{:08x}, Db Name='{}'",
        rh.db_id,
        rh.db_id,
        String::from_utf8_lossy(&rh.db_name),
    );
    println!(
        "	Activation ID={}=0x{:08x}",
        rh.activation_id, rh.activation_id,
    );
    println!(
        "	Control Seq={}=0x{:x}, File size={}=0x{:x}",
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
    println!(
        r#" descrip:"{}""#,
        String::from_utf8_lossy(&rh.description).replace('\0', "")
    );
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
    println!(
        " Disk cksum: 0x{:04x} Calc cksum: 0x{:04x}",
        rh.head.checksum, checksum
    );
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

    println!(
        " End-of-redo stream : {}",
        if rh.misc_flags.end_of_stream() {
            "Yes"
        } else {
            "No"
        }
    );
    println!(" {}", rh.misc_flags.protected_mode());
    println!(" Miscellaneous flags: 0x{:08x}", rh.misc_flags.into_bits());
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
    println!(" redo log key is {}", Hex::sequence(&rh.log_key));
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

fn print_record_head(i: usize, rba: &RBA, head: &RedoRecordHeadCommon, ts: OracleTimestamp) {
    println!(
        "{} - Thread:? RBA: 0x{:06x}.{:08x}.{:04x} LEN: 0x{:04x} VLD: 0x{:02x} CON_UID: {}",
        format!("REDO RECORD#{i}").bold().green(),
        rba.seq,
        rba.block_id,
        rba.offset,
        head.len,
        head.vld.into_bits(),
        head.con_uid,
    );
    println!(
        "SCN: 0x{:016x} SUBSCN:  {} {}",
        head.scn().full_scn(),
        head.sub_scn,
        OracleDateTime::from(ts),
    );
}

fn print_record(op: &Op<'_>) -> anyhow::Result<bool> {
    match op {
        Op::SegmentManagement(x) => x.dump(),
        Op::TransactionUndoManagement(x) => x.dump(),
        Op::Table(x) => x.dump(),
        _ => Ok(false),
    }
}

fn print_change(
    i: usize,
    force_print_vector: bool,
    change: &ChangeAndVector,
) -> anyhow::Result<()> {
    println!(
        "{} CON_ID:{} TYP:{} CLS:{} AFN:{} DBA:{} OBJ:{} SCN:0x{:016x} SEQ:{} OP:{}.{} ENC:{} RBL:? FLG:0x{:04x}",
        format!("CHANGE #{i}").green(),
        change.change.cont_id,
        change.change.type_.into_bits(),
        change.change.block_class,
        change.change.file_number,
        change.change.dba,
        change.change.obj(),
        change.change.scn.full_scn(),
        change.change.seq,
        change.change.layer,
        change.change.op,
        if change.change.type_.encrypted() {
            "1"
        } else {
            "0"
        },
        change.change.flags,
    );

    let printed = match change.parse() {
        Ok(o) => print_record(&o)?,
        Err(OpDataError::Encrypted) => {
            println!("{}", "CANNOT DUMP ENCRYPTED CHANGE!".bold().red());
            true
        }
        Err(e) => return Err(e)?,
    };

    if !printed || force_print_vector {
        if !printed {
            println!("{}", "UNRECOGNIZED VECTOR DATA".bold().yellow());
        }

        for (i, v) in change.vector.iter().enumerate() {
            let header = format!(
                "vector#{i}[{}{}]",
                v.size,
                if v.size == v.data.len() as _ {
                    "".to_string()
                } else {
                    format!("+{}", v.data.len() as u64 - v.size)
                }
            )
            .underline();
            print!(" {}", header);
            let ident = if v.ranges.len() > 1 {
                println!(
                    "\t{} {}",
                    format!("CROSSED BLOCK({} pieces)", v.ranges.len())
                        .bold()
                        .red(),
                    Hex::grouped(&v.data, 4)
                );
                format!("{}\t", " ".repeat(header.len()))
            } else {
                "\t".to_string()
            };

            let mut rest = &v.data[0..];
            for r in &v.ranges {
                let curr;
                (curr, rest) = rest.split_at((r.end - r.start) as _);
                println!(
                    "{}0x{:08X}..0x{:08X}: {}",
                    ident,
                    r.start,
                    r.end,
                    Hex::grouped(curr, 4)
                );
            }
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

pub fn read_redo(
    file: PathBuf,
    force_print_vector: bool,
    filter: Option<String>,
) -> anyhow::Result<()> {
    println!("DUMP OF REDO FROM FILE {file:?}");
    if let Some(ref f) = filter {
        println!("FILTER: {f}");
    }
    print_example_infos();

    let mut reader = RedoFileReader::from_file(file)?;
    reader.reload_header()?;

    let file_header = reader.file_header();
    let redo_file_header = reader.redo_file_header();
    let checksum = reader.read_file_header_checksum()?.0;
    print_header_info(file_header, redo_file_header, checksum);

    let mut i_record = 1usize;
    let mut begin_time = OracleTimestamp(0);
    while let Some((rba, record)) = eof_return_ok!(reader.begin_read_record())? {
        // pick redo record begin time if possible
        // only first record contains begin time (in Oracle 21.3.0 ee) and
        // begin time of records after first record keep unchanged
        if let Record::V2(v2) = record {
            begin_time = v2.begin_time;
        }

        let mut record_printed = false;
        let mut i_change = 1usize;
        while let Some(change) = eof_return_ok!(reader.read_change())? {
            // filter record
            if let Some(ref filter_expr) = filter {
                let mut variables = std::collections::HashMap::new();
                variables.insert("record", i_record as i64);
                variables.insert("change", i_change as i64);
                variables.insert("layer", change.change.layer as i64);
                variables.insert("op", change.change.op as i64);
                variables.insert("scn", change.change.scn.full_scn() as i64);

                if !filter::matches(filter_expr, &variables)? {
                    i_change += 1;
                    continue;
                }
            }

            // print record head if not printed
            if !record_printed {
                println!();
                match record {
                    Record::V1(v1) => print_record_head(i_record, &rba, &v1.common, begin_time),
                    Record::V2(v2) => {
                        print_record_head(i_record, &rba, &v2.common, v2.begin_time);
                        println!(
                            "(LWN RBA: 0x{:06x}.{:08x}.{:04x} LEN: 0x???????? NST: 0x???? SCN: 0x{:016x})",
                            rba.seq,
                            rba.block_id,
                            rba.offset,
                            v2.lwn_scn.full_scn(),
                        );
                    }
                }
                record_printed = true;
            }

            print_change(i_change, force_print_vector, &change)?;
            i_change += 1;
        }
        i_record += 1;
    }
    Ok(())
}
