# Block Cleanout Record (OP:4.1) Analysis

## Overview

Layer 4 (`BlockCleanOut`) op 1 is the standalone **block cleanout record**.
The same record also appears *embedded* in layer 11 changes whose `KtbRedo`
header has `op: F` and the cleanout bit set; there it follows the
`xid`/`uba` pair in `vector#1`.

```
CHANGE #1 ... OP:4.1 ...
Block cleanout record, scn:  0x00000000009edadd ver: 0x01 opt: 0x02 bigscn: Y compact: Y spare: 00000000, entries follow...
  itli: 2  flg: (opt=2 whr=1)  scn:  0x00000000009ed3fb
```

## Layout

### Head

Embedded (layer 11 `op: F`) records carry a 20-byte prefix:

```
unknown[20] | opt u8 | entries_count u8 | flags u8 | unknown u8 | scn[8]
```

Standalone `OP:4.1` records start directly at `opt` (total 12 bytes):

```
opt u8 | entries_count u8 | flags u8 | unknown u8 | scn[8]
```

`opt` printed as `0x02`, `flags` bit 0-1 = version, bit 2 = `bigscn`,
bit 3 = `compact`.

### Entries

Each entry is **8 bytes**:

```
itli u8 | flag u8 | scn[6]
```

`flag` is `opt` (2 bits) + `whr` (6 bits), printed as
`flg: (opt=%d whr=%d)`.

The SCN inside an entry is a **6-byte** `{base: u32, wrapper: u16}` pair, not
the usual 8-byte `Scn`. This matters whenever `entries_count >= 2`: reading the
entries with an 8-byte-SCN layout shifts every subsequent entry by two bytes.
The earlier `BlockCleanOutRecordEntry` (10 bytes) only happened to work for
single-entry records because the two extra bytes were zero.

### Tail

After the `entries_count` 8-byte entries come **two unknown bytes per entry**,
then a 4-byte `spare`:

```
total = head + 8 * n + 2 * n + 4
```

All observed `2*n` bytes are zero (`#{AMND}`: the field is not yet identified
and every sample, including multi-entry ones, is zero). `spare` is printed by
Oracle *without* a `0x` prefix: `spare: 00000000`.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `BlockCleanOutRecord = 1 !layers::layer4::BlockCleanOutRecord` |
| `crates/ointernal-parser/src/oracle/change/layers/layer4.rs` | 8-byte entries, `parse()` (embedded) / `parse_standalone()`, `FromCV` |
| `crates/ointernal-dumper/src/dump/dump_layer4.rs` | `Dump for BlockCleanOut` and the shared `Dump for BlockCleanOutRecord` |

`BlockCleanOutRecord` now exposes `opt`/`flags`/`scn` by value instead of a
`head` reference, so both head shapes share one printer. Layer 11's embedded
cleanout uses the same `Dump` implementation.

## Verification

```bash
for f in out/dumped/*/*/redo.dbf; do
  ./odumper dump "$f" | grep -A4 "Block cleanout record"
done
```

Every cleanout record emitted by the dumper (67 across the current archives,
including 2- and 4-entry records) matches the Oracle dump line-for-line.
Oracle additionally prints cleanout records nested inside undo records
(`KDO undo record` of `OP:5.1`), which this parser does not decode yet.
