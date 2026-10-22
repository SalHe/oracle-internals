# Extent-Map Redo (OP:14.4 `kteop`)

## Overview

`OP:14.4` is `kteop redo - redo operation on extent map`: an operation on a
segment's extent map (or on the segment header's extent bookkeeping). Oracle
always prints the banner line, then one operation-specific block:

```
kteop redo - redo operation on extent map
   SETHWM:       Highwater::  0x00429bca  ext#: 0      blk#: 1      ext size: 7
  #blocks in seg. hdr's freelists: 0
  #blocks below: 1
  mapblk  0x00000000  offset: 0
```

Every sample in the current archives is a `SETHWM` (`op = 7`) emitted while a
row append advances the segment high-water mark. The `TestOverwriteAppendValues`
run has four of them, and `TestOverwriteAppendSelect`, `TestOverwriteCtas` and
`TestOverwriteStatsHistogram` have one each.

The same payload is also logged *inside* a `ktudh` undo record (see
`docs/notes/07-layer14-18-reuse-and-extent.md`). There Oracle prints the
banner without a preceding newline, glued to the `pxid:` line.

## Payload layout

The payload starts with a 9-word header. Only the leading words are meaningful
for a given operation; the list operations append an inline array after the
first three words. Because the length depends on the operation, the parser
reads words on demand instead of mapping a fixed `#[repr(C)]` struct.

```
word 0 : operation (low byte) + flags (upper bits)
word 1..8 : operation-specific arguments
word 9.. : inline array for UNLINK / MVMAP / MOVAXT / MOVRET
```

The operation is the **low byte** of word 0. When the payload is embedded in a
`ktudh` record, word 0 is `0x00007f07`: the low byte is still `7` (`SETHWM`)
and `0x7f` is a flag byte. Standalone OP:14.4 changes use a zero upper part.

### `SETHWM` (7) word map

| Word | Field | Oracle output |
|------|-------|---------------|
| 0 | op | — |
| 1 | extent number | `ext#:` |
| 2 | block number in extent | `blk#:` |
| 3 | extent size | `ext size:` |
| 4 | high-water DBA | `Highwater::` |
| 5 | map block DBA | `mapblk` |
| 6 | map block offset | `offset:` |
| 7 | blocks on the segment header's freelists | `#blocks in seg. hdr's freelists:` |
| 8 | blocks below the high-water mark | `#blocks below:` |

Validated by patching a single word at a time in
`out/dumped/.../TestOverwriteAppendValues/redo.dbf` (vector at file offset
`0x27BC`, 36 bytes):

```
w1=0x1111                  -> ext#: 4369
w2=5, w3=9, w8=7           -> blk#: 5, ext size: 9, #blocks below: 7
w6=0x7777, w7=5            -> offset: 30583, #blocks in ... freelists: 5
w5=0x01000600 / 0x429bc8   -> mapblk  0x01000600 / 0x00429bc8
w4                         -> Highwater::  0x00429bca
```

The embedded sample in `TestOverwriteStatsHistogram`
(word 0 `0x00007f07`, words `4, 0, 0x80000000, 0x01000600, 0, 4, 0, 0`)
prints `Highwater:: 0x01000600 ext#: 4 blk#: 0 ext size: 128 ...
offset: 4`, matching the same map.

## Operation map

Patching word 0 through every value exposed each operation's output and which
words it reads. The probe used `w1=111, w2=222, w3=333, w4=0x00429bca,
w5=0x01000600, w6=666, w7=777, w8=888`.

| Op | Name | Output (verbatim) | Words |
|----|------|-------------------|-------|
| 0, 15, 16+ | — | `   DEFAULT: ???` | — |
| 1 | ADD | `   ADD: dba:0x6f len:222 at offset:333` | `dba=w1`, `len=w2`, `offset=w3` |
| 2 | DELETE | `   DELETE: entry:111` / `   shift back: dba:0xde len:333 ` | `entry=w1`, `dba=w2`, `len=w3` |
| 3 | LINKMAP | `   LINKMAP: nextmap:0x6f  clear at: 222` | `nextmap=w1`, `clear at=w2` |
| 4 | UNLINK | `   UNLINK: nextmap:0x6f extno:222` + `      dba:0x14d len:4365258` × `extno` | `nextmap=w1`, `count=w2`, pairs from `w3` |
| 5 | SETSTAT | `   SETSTAT: exts:111 blks:222 lastmap:0x14d mapcnt:4365258` | `w1..w4` |
| 6 | RESIZE | `   RESIZE: entry:111 delta:%` | `entry=w1`; `%` is literal |
| 7 | SETHWM | see above | `w1..w8` |
| 8 | UPDXNT | `   UPDXNT: extent:111 add:TRUE` | `extent=w1`, `add=(w2 != 0)` |
| 9 | MVMAP | `   MVMAP: extno:222, entries:111` + `      dba:... len:...` × `entries` | `entries=w1`, `extno=w2`, pairs from `w3` |
| 10 | CDOBJ | `   CDOBJ: new object number:111` | `w1` |
| 11 | MOVAXT | `  MOVAXT: offset:111 entries:222` + `  [i] fdba:0x... bdba:0x...` | `offset=w1`, `entries=w2`, pairs from `w3` |
| 12 | ADDAXT | `  ADDAXT: offset:111 fdba:x000000de bdba:0x0000014d` | `offset=w1`, `fdba=w2`, `bdba=w3` |
| 13 | MOVRET | `  MOVRET: offset:111 entries:222` + `  [i] ctime:333` | `offset=w1`, `entries=w2`, values from `w3` |
| 14 | ADDRET | `  ADDRET: offset:111 ctime:222` | `offset=w1`, `ctime=w2` |

Notes on the verbatim formats:

* `ADD`, `DELETE`, `LINKMAP`, `UNLINK`, `SETSTAT` and `MVMAP` print the DBA
  with `0x%x` (no zero padding); `MOVAXT`/`ADDAXT` use eight hex digits.
* `DELETE`'s second line ends with a space.
* `LINKMAP` has two spaces between `nextmap:0x..` and `clear at:`.
* `RESIZE` prints a literal `%` — Oracle's format string is broken.
* `ADDAXT` prints `fdba:x000000de`, i.e. the `0` of `0x` is missing (another
  broken format string).
* Ops 1–10 indent with three spaces; ops 11–14 with two.
* `UPDXNT`'s `add` flag is `TRUE` when word 2 is non-zero: with `w2=222` it
  printed `add:TRUE`, with `w2=0` it printed `add:FALSE`.
* `UNLINK`/`MVMAP`/`MOVAXT`/`MOVRET` read their count from the payload and
  loop that many times. Oracle happily reads past the end of a hand-patched
  vector (the probe printed 222 entries from a 36-byte payload); our dumper
  stops at the end of the vector instead.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `ExtentMapRedo = 4 !layers::layer14::ExtentMapRedo` |
| `crates/ointernal-parser/src/oracle/change/layers/layer14.rs` | `ExtentMapRedo` (payload bytes + word accessors), `ExtentMapOp` |
| `crates/ointernal-dumper/src/dump/dump_layer14.rs` | banner + per-operation `Dump` implementation |

## Verification

All six standalone `OP:14.4` blocks in the current archives match the Oracle
dump line-for-line, including trailing spaces:

```bash
for f in out/dumped/*/*/redo.dbf; do
  ./odumper dump "$f" | sed 's/\x1b\[[0-9;]*m//g' | grep -A5 'OP:14.4'
done
```

## Open items

* `OP:14.5` (`kteopu undo - undo operation on extent map`) is printed by
  Oracle inside the `ktubu` undo block:

  ```
  kteopu undo - undo operation on extent map
         segdba: 0x429bc8  class: 4  mapdba:0x429bc8  offset: 3
   rbr extent - dba: 0x0  nbk: 0x0
  ```

  It is not a separate change, so it needs to be parsed out of the 5.1 undo
  payload.
* The 5.2 `ktudh` dump should print its embedded kteop payload
  (`vector#1`, banner glued to the `pxid:` line) before `pdbuid:`.
