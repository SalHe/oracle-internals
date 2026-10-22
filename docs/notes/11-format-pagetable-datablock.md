# Format Pagetable Datablock (OP:13.21 `ktspbfredo`)

## Overview

`OP:13.21` formats a pagetable data block. Oracle prints three lines (the
`cscn` line only when a flag bit is set):

```
ktspbfredo - Format Pagetable Datablock 
Parent(l1) DBA:  0x00c122b8 typ: 2 objd: 78054 itls: 2 fmt_flag: 0 poff: 0
 cscn: 0x00000000009ef829 inc#:0
```

The banner ends with a space and `DBA:` is followed by two spaces. The 11
samples in `TestOverwriteStatsHistogram` cover two segment types (`typ: 1` and
`typ: 2`) and several object numbers.

## Payload layout

Two vectors:

| Vector | Size | Content |
|--------|------|---------|
| 0 | 20 bytes | block description |
| 1 | 16 bytes | creation SCN and incarnation |

### Vector 0

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `objd` (u32) | `objd:` |
| 4 | `typ` (u16) | `typ: typ & 0x7fff`; bit 15 gates the second line |
| 6 | `itls` (i16) | `itls:`, printed sign-extended to 64 bits |
| 8 | `fmt_flag` (u32) | `fmt_flag:` |
| 12 | `parent_dba` (u32) | `Parent(l1) DBA:` as `0x{:08x}` |
| 16 | `poff` (u8) | `poff:` |
| 17-19 | `#{AMND}` | — |

### Vector 1

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `cscn` (`Scn`, 8 bytes) | `cscn:` as the full 64-bit SCN |
| 8 | `inc` (u32) | `inc#:` |
| 12 | `#{AMND}` | — |

`Scn` is the usual 8-byte `{base, wrapper, marker}` structure, so the printed
`cscn` is not just the first word: the `wrapper`/`marker` logic in
`Scn::full_scn()` applies (e.g. `wrapper = 0x8001` yields a high word of
`0x00010000`).

## Probe evidence

Patches against the first sample's vectors at `0x7598` (vector 0) and `0x75AC`
(vector 1):

| Patch | Result |
|-------|--------|
| `0x7598` (objd) = `0x1111` | `objd: 4369` (change header `OBJ` unchanged) |
| `0x759C` (typ) = `0x0A` | `typ: 10` |
| `0x759D` = `0x81` / `0x01` | `typ: 258` both times, but only `0x81` prints `cscn` |
| `0x759D` = `0x40` / `0xC0` | `typ: 16386`; only `0xC0` prints `cscn` |
| `0x759E` = `0x07` | `itls: 7` |
| `0x759F` = `0xFF` | `itls: 18446744073709551362` (`0xff02` sign-extended) |
| `0x75A0` (fmt_flag) = `0x12345678` | `fmt_flag: 305419896` (full u32) |
| `0x75A4` (parent) = `0x01020304` | `Parent(l1) DBA:  0x01020304` |
| `0x75A8` (poff) = `0xFF` | `poff: 255` |
| `0x75A9`-`0x75AB` | no output change |
| `0x75AC` (cscn base) = `0x12345678` | `cscn: 0x0000000012345678` |
| `0x75B0` (cscn wrapper/marker) = `0x11223344` | `cscn: 0x00003344009ef829` |
| `0x75B4` (inc) = `0x55667788` | `inc#: 1432778632` |
| `0x75B8` | no output change |

Setting bit 15 of `typ` changes nothing else, which is why the printed type is
`typ & 0x7fff` while the flag only controls the second line.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `FormatPagetableDatablock = 21 !layers::layer13::FormatPagetableDatablock` |
| `crates/ointernal-parser/src/oracle/change/layers/layer13.rs` | `FormatPagetableDatablockHead`, `FormatPagetableDatablockTail`, wrapper |
| `crates/ointernal-dumper/src/dump/dump_layer13.rs` | `Self::FormatPagetableDatablock` branch + printer |

## Verification

All 11 blocks (33 payload lines) in `TestOverwriteStatsHistogram` match the
Oracle dump line-for-line, including the trailing space of the banner.
