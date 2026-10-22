# File Space Header Redo (OP:22.2 `ktfbhredo`)

## Overview

`OP:22.2` redoes a change to a locally managed tablespace file space header
(header block 2). Oracle prints a banner, an always-empty `Header Opcode:` line,
and a saved-operation summary:

```
ktfbhredo - File Space Header Redo:
Header Opcode: 
Save: Using Extent: 
Begin: 170952, Length: 8, Instance: 0 
```

The `Save:` prefix depends on a save-area disposition byte and the operation
name and extra line depend on the first payload byte:

```
ktfbhredo - File Space Header Redo:
Header Opcode: 
Save: File Resize: 
New End: 170952, New Size: 8 
```

## Payload layout

One vector of 2 or 12 bytes:

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `op` (u8) | operation name after the prefix |
| 1 | `save` (u8) | `Save` (0), `NoSave` (1), `Clear` (anything else) |
| 2 | `instance` (u16) | `Instance:` of the `Begin:` line |
| 4 | `begin` (u32) | `Begin:` / `New End:` / `First:` / `MaxSize:` / `BeginBlock:` |
| 8 | `length` (u32) | `Length:` / `New Size:` / `Free:` / `Increment:` |
| 12 | — | vector ends |

A 2-byte vector carries only `op` and `save` (observed for `NoPending`). The
reader keeps the 4-byte alignment padding, so `instance` is still taken from
the two padding bytes — Oracle reads it from the same place (`Instance: 64`
when the padding is `40 00`). `begin`/`length` have no payload bytes at all;
Oracle prints whatever precedes the vector in the redo record (e.g. `Begin:
104, Length: 2` read from the `ktudh` vector of the preceding change), which
cannot be reproduced from the change payload, so a patched short vector differs
there. Natural short vectors always carry op 0 and print no extra line.

## Operation table

The prefix line is `{Save|NoSave|Clear}: <name>`:

| `op` | Name | Extra line |
|------|------|------------|
| 0 | `No Pending Op ` | — |
| 1 | `Using Extent: ` | `Begin: {}, Length: {}, Instance: {} ` |
| 2 | `Freeing Extent: ` | `Begin: ...` |
| 3 | `Free Extent: ` | `Begin: ...` |
| 4 | `Use Extent: ` | `Begin: ...` |
| 5 | `File Resize: ` | `New End: {}, New Size: {} ` |
| 6 | `Stats On: ` | `First: {}, Free: {} ` |
| 7 | `File AutoExtend On: ` | `MaxSize: {}, Increment: {} ` |
| 8 | `File AutoExtend Off: ` | — |
| 9 | `Stats Off: ` | — |
| 0x10 | `Invalidate Range: ` | `BeginBlock: {}, Length: {} ` |
| other | `Unknown OPcode: ` | — |

All extra lines end with a trailing space; `No Pending Op ` also does, but has
no colon.

## Probe evidence

Patches against `TestOverwriteStatsHistogram` (vector-0 data offsets in
parentheses). Note that `./olabs redo dump -r` values are literal hex bytes
unless prefixed with `u16:`/`u32:`.

| Patch | Result |
|-------|--------|
| `op=1`, `save=0` (natural) | `Save: Using Extent:` + `Begin: ... Instance: 0` |
| `op=0`, `save=1` (`0x4288`) | `NoSave: No Pending Op ` |
| `op=0`, `save=2` (`0x6fdc`) | `Clear: No Pending Op ` |
| `save=3/4/0x7f/0x80/0xff` | always `Clear` |
| `op=2..9` (`0x417c`, `0x4c58`, `0x6ee0`, `0xadec`) | names and extra lines above |
| `op=0x10` | `Save: Invalidate Range: ` + `BeginBlock: 74368, Length: 8 ` |
| `op=0x0f/0x11/0x12` | `Save: Unknown OPcode: ` |
| `instance=0xCDAB` (`0x177a`) | `Instance: 52651` |
| `begin=0x11223344` (`0x177c`) | `Begin: 287454020` / `New End: 287454020` |
| `length=0x55667788` (`0x1780`) | `Length: 1432778632` / `New Size: 1432778632` |
| short vector, `op=1` (`0x4288`) | `Begin: 104, Length: 2, Instance: 0`; patching `0x4220`/`0x4224` changes begin/length, so those come from the preceding change's `ktudh` vector, not the payload |
| short vector padding (`0xaf02` = `0x40`) | `Instance: 64` |

The `Header Opcode:` field is empty for every value of the payload bytes, the
change-header flags and the change-header unknown dword, so its source is not
in the change payload; it is printed as a constant.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `FileSpaceHeaderRedo = 2 !layers::layer22::FileSpaceHeaderRedo` |
| `crates/ointernal-parser/src/oracle/change/layers/layer22.rs` | `FileSpaceHeaderSave`, `FileSpaceHeaderOp`, `FileSpaceHeaderRedoHead`, byte-slice wrapper |
| `crates/ointernal-dumper/src/dump/dump_layer22.rs` | banner, prefix/name dispatch, per-op extra line |

## Verification

All 10 natural blocks match the Oracle dump line-for-line, including trailing
spaces: 2 in `TestOverwriteCtas` and 8 in `TestOverwriteStatsHistogram`.

## Open items

- The `Header Opcode:` value is not in the payload; no sample prints anything
  there.
- `OP:22.3` (`ktfbhundo`) and the `Save:`/`Begin:` lines emitted for ktfbh undo
  embedded in `OP:5.24` (observed as `Save: Free Extent:`) still need decoding;
  the same operation table is expected.
- `OP:22.5` (`ktfbbredo`) shares the `Begin:` line format and is the next
  layer-22 target.
