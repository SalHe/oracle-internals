# Layer 14/18 Redo and the `ktudh` Extension (OP:14.1, 14.2, 18.3)

## Overview

| Code | Oracle name | Meaning | Payload |
|------|-------------|---------|---------|
| 14.1 | `ktecush` | clear extent control lock | none |
| 14.2 | `ktelk` | lock extent (map) | `Xid` (8 bytes) |
| 18.3 | reuse redo entry | object / range reuse marker | 24 bytes |

All three show up in `TestOverwriteAppendValues` (`test_overwrite_row_piece.py`):
each appended row produces 14.2 → 14.4 → 14.1 around the 18.3 reuse marker.

## OP:14.2 — lock extent

`vector#0` is the transaction `Xid`, exactly the layer 5 `Xid` layout:

```
usn u16 | slot u16 | sequence u32
```

Oracle prints (two spaces after `xid:`):

```
ktelk redo: xid:  0x0009.009.0000048f
```

## OP:14.1 — clear extent control lock

The change carries **no vector at all**. Oracle prints the constant:

```
ktecush redo: clear extent control lock
```

Because there is nothing to parse, the opcode is registered without a payload
type and the printer lives on the layer-level `Dump` impl.

## OP:18.3 — reuse redo entry

Oracle prints two lines:

```
Reuse redo entry
Prewarm range reuse: pdb = 1 tsn=0 base=0x00429bca nblks=1
```

The single vector is 24 bytes / six `u32` fields:

```
type_ u32 @0 | tsn u32 @4 | objd u32 @8 | base u32 @12 | nblks u32 @16 | pdb u32 @20
```

`type_` selects the output line:

| `type_` | Output |
|---------|--------|
| 1 | `Object reuse: pdb={pdb}, tsn={tsn} objd={objd}` (uses `objd` @8) |
| 2 | `Range reuse: pdb = {pdb} tsn={tsn} base=0x{base:08x} nblks={nblks}` |
| 4 | `Prewarm range reuse: ...` (same fields as range) |
| other (0, 3, 5, ...) | `Local range reuse: ...` (same fields as range) |

`objd` and `base` share neither offset nor meaning: `Object` reads @8 and the
range variants read @12. Verified by single-field byte modification:

```
0x4D20 u32:1, 0x4D28 u32:0xAAAA  -> Object reuse: pdb=1, tsn=0 objd=43690
0x4D20 u32:1, 0x4D2C u32:0xBBBB  -> Object reuse: pdb=1, tsn=0 objd=0
0x4D24 u32:0x55, 0x4D30 u32:7, 0x4D34 u32:0x9999
                                 -> Prewarm range reuse: pdb = 39321 tsn=85 base=0x00429bca nblks=7
0x4D20 u32:0                    -> Local range reuse: pdb = 1 tsn=0 base=0x00429bca nblks=1
```

These changes are *media recovery markers*: Oracle's change header reads
`CHANGE #1 MEDIA RECOVERY MARKER CON_ID:1 ... OP:18.3` while our generic
header still prints `TYP:6 ...` (pre-existing header gap, unchanged here).

## Related fix: `ktudh` pdb uid vector (OP:5.2)

`TestOverwriteStatsHistogram` contains a 5.2 change whose `vector#1` is a
36-byte embedded extent-map redo (Oracle appends `kteop redo - redo operation
on extent map` to the `ktudh` line without a newline) and whose `vector#2` is
the 4-byte pdb uid:

```
vector#0[32]  ktudh
vector#1[36]  kteop redo (extent map, OP:14.4 payload)
vector#2[4]   pdb uid
```

`UndoHeaderRedo` previously assumed the pdb uid was always `vector#1`, which
made the whole dump abort with `Size { got: 4 }`. It now takes the *trailing*
vector when the `pdb_uid` flag is set. Every other sample still has
`[32, 4]`, so the visible output is unchanged except that the dump no longer
stops early on this archive.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `LockExtent = 2 !layers::layer14::LockExtent`, `ObjectRangeReuse = 3 !layers::layer18::ReuseRedoEntry` |
| `crates/ointernal-parser/src/oracle/change/layers/layer14.rs` | `LockExtent` (`&Xid`) |
| `crates/ointernal-parser/src/oracle/change/layers/layer18.rs` | `ReuseRedoEntryHead` (24 bytes) + `ReuseType` |
| `crates/ointernal-parser/src/oracle/change/layers/layer5.rs` | pdb uid taken from the trailing vector |
| `crates/ointernal-dumper/src/dump/dump_layer14.rs` | 14.1 constant + 14.2 line |
| `crates/ointernal-dumper/src/dump/dump_layer18.rs` | `Reuse redo entry` + the four reuse lines |

## Verification

```bash
for f in out/dumped/*/*/redo.dbf; do
  ./odumper dump "$f" | grep -E '^(ktelk|ktecush|Reuse redo entry)'
done
```

Every 14.1/14.2/18.3 section in the current archives matches the Oracle dump
line-for-line. The `kteop` (14.4) payload is decoded in
`docs/notes/08-extent-map-redo.md`.
