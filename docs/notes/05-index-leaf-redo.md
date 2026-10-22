# Index Leaf Row Redo (kdxlin / kdxlde) Analysis

## Overview

Layer 10 (`Index`) carries the index redo. The two most frequent operations are:

| Code | Oracle name | Meaning |
|------|-------------|---------|
| 10.2 | `kdxlin` | insert leaf row |
| 10.4 | `kdxlde` | delete leaf row |

Both are "affects a leaf block" changes; they differ only in whether the redo
carries the inserted keys.

## Reproducers

* `oracle_internals/cases/test_index_insert_leaf_row.py`
  (`TestIndexInsertLeafRow`): `INSERT` one row at a time into an indexed table.
  Each insert emits one single-key 10.2.
* `oracle_internals/cases/test_index_array_insert.py`
  (`TestIndexArrayInsertAndDelete`): `INSERT ... SELECT` of 10 rows goes through
  the array-insert path and folds all 10 index keys into one ARRAY 10.2;
  `DELETE ... WHERE c1 IN (1,2,3)` follows with an ARRAY 10.4.

## Vector Layout

`vector#0` is the usual `KtbRedo`/`RowOperation` block (same layout as layer
11). `vector#1` is the index metadata. Remaining vectors carry the keys.

### vector#1: SINGLE (6 bytes)

```
itl u8 | flags u8 | sno u16 | row_size u16
```

### vector#1: ARRAY (12 bytes + slots)

Bit `0x20` of `flags` selects this shape:

```
itl u8 | flags u8 | sno u16 | row_size u16 | unknown u16 | num_keys u32 | slots [u16]
```

Oracle prints only `num_keys` slots. On ARRAY 10.4 the vector was seen to carry
one slot more than `num_keys` (a trailing `0`), so the dumper clamps the list.

### key vectors

| Shape | vector#2 | vector#3 |
|-------|----------|----------|
| SINGLE 10.2 | the single key | — |
| ARRAY 10.2 | all keys concatenated | per-key sizes (`u16` each) |
| ARRAY 10.4 | — | — |

A 10.4 change deletes by `(itl, slot)` only, so it has no key vectors at all.

## Oracle Dump Output

SINGLE 10.2:

```
index redo (kdxlin):  insert leaf row
KTB Redo 
op: 0x01  ver: 0x01  
compat bit: 4 (post-11) padding: 1
op: F  xid:  0x000a.01e.0000046f    uba: 0x0100143b.0103.09
REDO: 0x0 SINGLE / -- / -- 
itl: 2, sno: 0, row size 14
insert key: (10):  02 c1 02 06 00 42 9b c9 00 00
```

ARRAY 10.2:

```
REDO: 0x20 ARRAY / -- / -- 
itl: 2, sno: 0, row size 140
number of keys: 10
slots: 
 0 1 2 3 4 5 6 7 8 9
insert key: (100): 
 02 c1 02 06 00 42 9b c9 00 00 ...
each key size is: 
 10 10 10 10 10 10 10 10 10 10
```

Formatting rules (spaces are significant):

* Two spaces after `(kdxlin):` / `(kdxlde):`.
* `slots: ` and `each key size is: ` end with a trailing space, and the values
  go on the next line prefixed with a single space.
* `insert key: ({total}):` is followed by two spaces when the key fits on one
  line (≤ 25 bytes) and by one space + line break otherwise; bytes are spaced
  and wrapped every 25 bytes, same as `col` output.
* ARRAY 10.4 prints only `number of keys` and `slots`; it never prints keys.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `InsertLeafRow = 2 !layers::layer10::KdxRedo`, `DeleteLeafRow = 4 !layers::layer10::KdxRedo` |
| `crates/ointernal-parser/src/oracle/change/layers/layer10.rs` | `KdxRedo { op, meta }`, `KdxMetaSingle` / `KdxMetaArray`, key split by the v3 size vector |
| `crates/ointernal-dumper/src/dump/dump_layer10.rs` | `Dump for Index` + `Dump for KdxRedo` |

## Verification

```bash
./olabs collect -- -k TestIndexInsertLeafRow -s -v
./olabs collect -- -k TestIndexArrayInsertAndDelete -s -v

./odumper dump out/dumped/oracle_internals.cases.test_index_array_insert/\
TestIndexArrayInsertAndDelete/redo.dbf --filter 'layer == 10'
```

SINGLE 10.2, ARRAY 10.2 and ARRAY 10.4 output match the Oracle dump
line-for-line (modulo the pre-existing `RBL:?` in the change header).
