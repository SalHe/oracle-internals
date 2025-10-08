use std::path::PathBuf;

use chrono::DateTime;
use clap::Parser;
use ointernal_parser::oracle::{
    file::FileHeaderBlock,
    reader::{RBA, ReaderError, Record, RedoFileReader},
    redo::{OracleTimestamp, RedoFileHead, RedoRecordHeadCommon},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ODumper {
    #[arg()]
    redo: PathBuf,
}

const DATETIME_FORMAT: &str = "%m/%d/%y %H:%M:%S";

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

fn format_timestamp(ts: OracleTimestamp) -> String {
    DateTime::from_timestamp_secs(ts.real_timestamp() as _)
        .unwrap()
        .format(DATETIME_FORMAT)
        .to_string()
}

fn print_header_info(fh: &FileHeaderBlock, rh: &RedoFileHead) {
    println!(" FILE HEADER:");
    println!("	Compatibility Vsn = {}=0x{:08X}", rh.vsn, rh.vsn,);
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
    println!("	Control Seq={}=0x{:04x}", rh.control_seq, rh.control_seq,);
    println!(
        "	File Number=1, Blksiz={}, File Type=2 LOG",
        fh.header.block_size
    );
    println!(r#" descrip:"{}""#, String::from_utf8_lossy(&rh.description));
    println!(
        " thread: 1 nab: {:x} seq: 0x{:08x} hws: 0x{:x} eot: 0 dis: 0", // TODO other fields?
        rh.nab, rh.head.seq, rh.hws,
    );
    println!(
        " resetlogs count: 0x{:08x} scn: 0x{:016x}",
        rh.reset_logs_count,
        rh.reset_log_scn.to_u64(),
    );
    println!(
        " prev resetlogs count: 0x{:08x} scn: 0x{:16x}",
        rh.prev_reset_logs_count,
        rh.prev_reset_logs_scn.to_u64(),
    );
    println!(
        " Low  scn: 0x{:016x} {}",
        rh.low_scn.to_u64(),
        format_timestamp(rh.low_scn_timestamp)
    );
    println!(
        " Next scn: 0x{:016x} {}",
        rh.next_scn.to_u64(),
        format_timestamp(rh.next_scn_timestamp),
    );
    println!(
        " Enabled scn: 0x{:016x} {}",
        rh.enabled_scn.to_u64(),
        format_timestamp(rh.enabled_scn_timestamp),
    );
    println!(
        " Thread closed scn: 0x{:016x} {}",
        rh.thread_closed_scn.to_u64(),
        format_timestamp(rh.thread_closed_scn_timestamp),
    );
    println!(" Real next scn: 0xffffffffffffffff"); // TODO ?source
    println!(" Disk cksum: 0x{:04x} Calc cksum: 0x????", rh.head.checksum); // TODO calc checksum
    println!(" Terminal recovery stop scn: 0x0000000000000000"); // ?
    println!(" Terminal recovery  01/01/1988 00:00:00"); // ?
    println!(" Most recent redo scn: 0x0000000000000000"); // ?
    println!(" Largest LWN: ? blocks");
    println!(" End-of-redo stream : ??");
    println!(" Unprotected mode???");
    println!(" Miscellaneous flags: 0x{:08x}", rh.misc_flags);
    println!(" Miscellaneous second flags: 0x{:08x}", rh.misc_flags2);
    println!(" Thread internal enable indicator: thr: ?, seq: ? scn: 0x????????????????");
    println!(" Zero blocks: ?");
    println!(" Format ID is ??");
    println!(" redo log key is {}", hex::encode(rh.log_key));
    println!(" redo log key flag is ??"); // ?
    println!(" DAX High watermark block number: ?"); //?
    println!(" DAX flags: 0x????"); // ?
    println!(" DAX checksum: 0x????"); // ?
    println!(" Enabled redo threads: ? "); // ?
}

fn print_record_head(rba: RBA, head: &RedoRecordHeadCommon, ts: OracleTimestamp) {
    println!(
        "SCN: 0x{:016x} SUBSCN:  {} {}",
        head.scn.base,
        head.scn.wrapper, // TODO verify
        format_timestamp(ts),
    );
    println!(
        "(LWN RBA: 0x{:06x}.0x{:08x}.0x{:04x} LEN: 0x???????? NST: 0x???? SCN: 0x{:016x})",
        rba.seq, rba.block_id, rba.offset, head.scn.base,
    );
}

fn read_redo(file: PathBuf) -> Result<(), ReaderError> {
    println!("DUMP OF REDO FROM FILE {file:?}");
    print_example_infos();

    let mut reader = RedoFileReader::from_file(file)?;
    reader.reload_header()?;

    let file_header = reader.file_header();
    let redo_file_header = reader.redo_file_header();
    print_header_info(file_header, redo_file_header);

    let mut i_record = 1usize;
    while let Some((rba, record)) = reader.begin_read_record()? {
        // println!("Record#{i_record}: {record:?}");
        println!();
        println!(
            "REDO RECORD#{i_record} - Thread: 1 RBA: 0x{:06x}.0x{:08x}.0x{:04x}",
            rba.seq, rba.block_id, rba.offset
        );

        match record {
            Record::V1(v1) => print_record_head(rba, &v1.common, OracleTimestamp(0)),
            Record::V2(v2) => print_record_head(rba, &v2.common, v2.begin_time),
        }

        let mut i_change = 1usize;
        while let Some(change) = reader.read_change()? {
            println!(
                "CHANGE #{i_change} CON_ID:? TYP:{} CLS:{} AFN:{} DBA:{:?} OBJ:{} SCN:{} SEQ:{} OP:{}.{} ENC:? RBL:? FLG:0x????",
                change.change.type_,
                change.change.block_class,
                change.change.file_number,
                change.change.dba,
                change.change.obj(),
                change.change.scn.to_u64(),
                change.change.seq,
                change.change.layer,
                change.change.op,
            );

            for (i, v) in change.vector.iter().enumerate() {
                println!("       vector#{i} {}", hex::encode(v));
            }

            i_change += 1;
        }
        i_record += 1;
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = ODumper::parse();

    match read_redo(args.redo) {
        Ok(_) => {}
        Err(ReaderError::Eof) => println!("END OF REDO DUMP"),
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
