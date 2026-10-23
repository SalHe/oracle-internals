# Update L/HWM, HHWM Redo (OP:13.28 `ktspfhw`)

## Overview

`OP:13.28` updates the low/high water marks of a segment. The single vector is
80 bytes: a selector dword followed by two 32-byte HWM descriptors and two
`lfdba` values. The selector (`kind`) decides which sections Oracle prints;
most values print nothing at all.

```
Both the HWMs
Low HWM
      Highwater::  0x00c12283  ext#: 0      blk#: 3      ext size: 8
  #blocks in seg. hdr's freelists: 0
  #blocks below: 0
  mapblk  0x00000000  offset: 0
lfdba:  0x00c12280
High HWM
      Highwater::  0x00c12283  ext#: 0      blk#: 3      ext size: 8
  #blocks in seg. hdr's freelists: 0
  #blocks below: 0
  mapblk  0x00000000  offset: 0
lfdba:  0x00c12280 CHANGE #2 ...
```

## Payload layout

One 80-byte vector:

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `kind` (u32) | selector, see below |
| 4 | High HWM descriptor (32 B) | `High HWM` section / `hint dba` / `Allocation Area` |
| 36 | Low HWM descriptor (32 B) | `Low HWM` section |
| 68 | `low_lfdba` (u32) | `lfdba:` of the `Low HWM` section |
| 72 | `high_lfdba` (u32) | `lfdba:` of the last printed section |
| 76 | unknown (u32) | `#{AMND}`: equals `high_lfdba + 1` in every sample, no output effect |

### HWM descriptor (32 bytes)

The recorded order differs from the printed order. `OP:13.22`
(`Redo on Level1 Bitmap Block`) carries the same descriptor after a 4-byte
header.

| Offset | Field | Printed as |
|--------|-------|------------|
| 0 | `ext` | `ext#` |
| 4 | `blk` | `blk#` |
| 8 | `ext_size` | `ext size` |
| 12 | `dba` | `Highwater:: 0x%08x` |
| 16 | `mapblk` | `mapblk  0x%08x` |
| 20 | `offset` | `offset` |
| 24 | `freelists` | `#blocks in seg. hdr's freelists` |
| 28 | `below` | `#blocks below` |

## Selector table

| `kind` | Output |
|--------|--------|
| 1 | `Lasts in Header` + `LF: {}, LS: {}, LT: {}, FT: {}` from High +0/+4/+8/+12 |
| 2 | `High HWM` + High descriptor + `high_lfdba` |
| 3 | `Low HWM` + Low descriptor + `low_lfdba` |
| 4 | `Both the HWMs` + Low section (newline-terminated) + High section |
| 6 | `hint dba:  0x%08x` from High +0 |
| 9 | `Update Express Allocation Area` + `Allocation Area : EMPTY\|FULL` |
| 0, 5, 7, 8, 10+ | nothing at all |

For selector 9 Oracle only looks at the low byte of High +0: zero prints
`EMPTY`, anything else prints `FULL`.

## Exact formatting

- `      Highwater::  0x%08x  ext#: %-6d blk#: %-6d ext size: %-6d`
  — `ext#` and `blk#` are a 6-wide left-justified field **plus one explicit
  space**; `ext size` is a 6-wide field with no explicit space.
- `  #blocks in seg. hdr's freelists: %-6d`
- `  #blocks below: %-6d`
- `  mapblk  0x%08x  offset: %-6d`
- `lfdba:  0x%08x` when the section is followed by another section, and
  `lfdba:  0x%08x ` (one trailing space, no newline) when it is the last
  section of the change.

The open final line is Oracle's: a following `CHANGE #n` header is glued to it
as `lfdba:  0x... CHANGE #2`, and when the change is the last of a record the
record-separator line terminates it, leaving two trailing spaces before the
newline.

### Record separator

Every `REDO RECORD` is preceded by a line holding a single space (`" \n"`).
When the previous change left its line open (selector 2/3/4/9), that marker
also terminates it and adds one more space. Our dumper therefore prints
`" "` instead of an empty line in `read_redo`.

## Probe evidence

Patches against the first selector-4 sample
(`TestOverwriteStatsHistogram/redo.dbf`, record #48, vector at `0x446C`..`0x44BC`):

| Patch | Result |
|-------|--------|
| payload +4..+32 and +36..+64 with distinct values | `Low HWM` reads +36, `High HWM` reads +4 |
| +0x44B0 | `Low HWM` `lfdba` |
| +0x44B4 | `High HWM` `lfdba` |
| +0x44B8 | no output change (`#{AMND}`) |
| kind = 0, 5, 7, 8, 10, 16..20, 24, 32, 64, 100, 128, 256 | no output |
| kind = 1 | `Lasts in Header` + `LF/LS/LT/FT` from High +0/+4/+8/+12 |
| kind = 2/3/4/6 | the sections listed in the table above |
| kind = 9 with High +0 = `0x100`/`0x200`/`0x10000`/`0xff00` | `EMPTY` |
| kind = 9 with High +0 = `0x2` | `FULL` |
| +0x4478 = `0x22222222` | `ext size: 572662306` with no trailing space → `%-6d` |
| +0x4474 = `0x22222222` | `blk#: 572662306 ` (one space) → `%-6d ` |
| +0x4488/+0x448C/+0x4484 = 9-digit values | no trailing space → `%-6d` |
| kind = 4 on a non-last change (#104) | Low `lfdba` newline-terminated, High `lfdba` glued to `CHANGE #2` |

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `UpdateLhwmHhwm = 28 !layers::layer13::UpdateLhwmHhwm` |
| `crates/ointernal-parser/src/oracle/change/layers/layer13.rs` | `UpdateLhwmHhwmOp`, `Hwm`, `UpdateLhwmHhwmData`, `UpdateLhwmHhwm` |
| `crates/ointernal-dumper/src/dump/dump_layer13.rs` | `SegmentManagement` branch, `dump_hwm` helper |
| `crates/ointernal-dumper/src/dump/mod.rs` | record separator prints `" "` |

## Verification

All 9 natural `OP:13.28` blocks in `TestOverwriteStatsHistogram` match the
Oracle dump byte-for-byte (sections of 424/424/204/205/424/204/205/204/205
bytes), including the glued `CHANGE #2` header and the trailing spaces.
`TestOverwriteCtas` matches on all 43 record separators.

## Open items

- `OP:13.22` reuses the same `Hwm` descriptor and is still unimplemented.
- The meaning of payload +76 is unknown (`#{AMND}`); it tracks `high_lfdba + 1`
  but has no output effect.
