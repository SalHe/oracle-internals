# ORP (OverwriteRowPiece) Redo Operation Analysis

## Overview

**ORP** (`KdoOp::ORP`) is redo change **11.6**. Oracle's dump calls it:

```
KDO Op code: ORP row dependencies Disabled
```

Unlike IRP/URP/DRP, ORP rewrites an existing row piece *in place*: the whole piece
is replaced, so the redo carries a complete row piece rather than a column diff.

Two shapes are observed in practice:

| Shape | `cc` | payload | meaning |
|-------|------|---------|---------|
| forwarding address | 0 | `nrid` only | the head piece is overwritten with a pointer to the migrated row |
| full row | > 0 | one vector per column | the entire row piece is rewritten |

## How to Reproduce

### Forwarding-address shape (user SQL)

`oracle_internals/cases/test_overwrite_row_piece.py::TestOverwriteRowMigration`
fills a block with short rows, then grows one row past the free space:

```sql
create table t_orp_mig (id number, v varchar2(4000), filler char(200)) pctfree 0;
insert into t_orp_mig select level, rpad('x', 700, 'x'), rpad('f', 200, 'f')
  from dual connect by level <= 8;
-- forces row migration:
update t_orp_mig set v = rpad('y', 3000, 'y') where id = 1;
```

The archive contains 11.2 (IRP into the new block), 11.4 (LKR) and 11.6:

```
CHANGE #10 ... DBA:0x00429bc9 ... OP:11.6 ENC:0 RBL:0 FLG:0x0000
KTB Redo 
op: 0x02  ver: 0x01  
compat bit: 4 (post-11) padding: 1
op: C  uba: 0x01000e56.0117.06
KDO Op code: ORP row dependencies Disabled
  xtype: XA flags: 0x00000000  bdba: 0x00429bc9  hdba: 0x00429bc8
itli: 2  ispac: 0  maxfr: 4863
tabn: 0 slot: 0(0x0) size/delt: 9
fb: --H----- lb: 0x2  cc: 0
nrid:  0x00429bca.0
null: 
```

### Full-row shape

A minimal user-SQL reproducer was not found. Direct-path `INSERT` (`append`,
`append_values`) and `CREATE TABLE AS SELECT` emit 11.2/11.3/11.5, and an UPDATE
of every column still emits 11.5 (URP). The full-row ORPs used for validation
come from dictionary maintenance (`SYS.HIST_HEAD$`, object 68) triggered by
`DBMS_STATS` in archive `1_61`; their embedded undo records are ORP as well
(`opc: 11.1`).

```
KDO Op code: ORP row dependencies Disabled
  xtype: XA flags: 0x00000000  bdba: 0x0040b1f6  hdba: 0x00400268
itli: 2  ispac: 24  maxfr: 4863
tabn: 0 slot: 131(0x83) size/delt: 77
fb: --H-FL-- lb: 0x2  cc: 18
null:
01234567890123456789012345678901234567890123456789012345678901234567890123456789
------------------
col  0: [ 4]  c3 08 4c 3b
...
col 17: [ 2]  c1 07
```

## Vector Layout

ORP uses the standard `KtbRedo` three-level vector scheme:

| Vector | Content |
|--------|---------|
| `vector#0` | `RowOperation` (16 bytes). `op: C`, `start_from_8: true`, UBA at offset 8 |
| `vector#1` | `RPMeta` (16 bytes) followed by the row-piece payload |
| `vector#2..` | one vector per column (`cc` vectors), same as IRP |

`vector#1` example (full-row sample, 51 bytes):

```
f6b14000 68024000 ff120601 02180000 2c021200 fd7f0000 3048c629
00000000 00000000 f3710000 4d008300 00000000 c00000ff
```

`RPMeta` (16 bytes):

| Offset | Size | Field | Value |
|--------|------|-------|-------|
| 0 | 4 | `bdba` | `0x0040b1f6` |
| 4 | 4 | `hdba` | `0x00400268` |
| 8 | 2 | `maxfr` | `0x12ff` = 4863 |
| 10 | 1 | `op` (`IRPMetaByte1`) | `0x06` = ORP |
| 11 | 1 | `xtype` (`IRPMetaByte2`) | `0x01` = XA |
| 12 | 1 | `itli` | 2 |
| 13 | 1 | `ispac` | 24 |
| 14 | 2 | unknown | 0 |

Payload: **byte-for-byte identical to `RPMetaIRP`** (IRP's payload). Reading from
`RPMeta.rest` (offset 16):

| Offset | Size | Field | Value |
|--------|------|-------|-------|
| 0 | 1 | `fb_flags` | `0x2c` = `--H-FL--` |
| 1 | 1 | `lb` | 2 |
| 2 | 1 | `column_count` | 18 |
| 3 | 1 | `cki` | 0 (only when `fb_flags.c()`) |
| 4 | 8 | unknown | |
| 12 | 4 | `nrid_dba` | 0 |
| 16 | 2 | `nrid_slot` | 0 |
| 18 | 6 | unknown | |
| 24 | 2 | `delta_size` | 77 (dumped as `size/delt`) |
| 26 | 2 | `slot` | `0x83` = 131 |
| 28 | 1 | `tabn` | 0 |
| 29 | *cc/8 rounded up* | null bitmap | all `-` here |

For the forwarding-address shape `column_count == 0`, `delta_size == 9` and
`nrid_dba`/`nrid_slot` point at the new piece
(`nrid:  0x00429bca.0`); there are no column vectors and no null bitmap bytes.

## Oracle Dump Formatting Rules

* `tabn: {tabn} slot: {slot}(0x{slot:x}) size/delt: {delta_size}`
* `fb: {flags} lb: 0x{lb:x}  cc: {cc}` — flags are printed as `KCHDFLPN`,
  with `-` for unset bits. When the `C` bit (cluster) is set, Oracle appends
  ` cki: N`.
* `nrid:  0x{dba:08x}.{slot:x}` only when the nrid is non-zero.
* `null:` followed by an 80-column ruler and a mask whose length is exactly
  `cc` (`-` non-null, `N` null). The ruler is printed even when `cc < 80`.
* `cc == 0` is special: Oracle prints `null: ` **with a trailing space** and no
  ruler/mask line.
* Column lines: `col {i:>2}: [{len:>2}]  {hex bytes}` (`*NULL*` when the null
  bitmap says null).

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `OverwriteRowPiece = 6 !layers::layer11::KtbRedo` |
| `crates/ointernal-parser/src/oracle/change/layers/layer11.rs` | `RPMetaRest::ORP(RPMetaIRPWrapper<'a>)`, parsed with `FromBytes::ref_from_bytes(&self.rest)` |
| `crates/ointernal-dumper/src/dump/dump_layer11.rs` | `Table::OverwriteRowPiece` dispatch and a shared `dump_row_piece()` used by both IRP and ORP |

Since the payload is identical to IRP, ORP reuses `RPMetaIRP`,
`RPMetaIRPWrapper`, and `col_data()` unchanged.

## Verification

```bash
./olabs collect -- -k TestOverwriteRowMigration -s -v

# Oracle ground truth
grep -A30 "OP:11.6" out/dumped/.../TestOverwriteRowMigration/redo.txt

# Parser output
./odumper dump out/dumped/.../TestOverwriteRowMigration/redo.dbf \
  --filter 'layer == 11 && op == 6'
```

Every field matches; the only difference is the pre-existing `RBL:0` vs
`RBL:?` in the change header. The full-row shape was cross-checked against all
7 ORP changes of archive `1_61` (5 pointer-only with `nrid` `.31`-`.34`, 2
full-row with 18 columns), including all column bytes.
