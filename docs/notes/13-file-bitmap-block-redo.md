# File BitMap Block Redo (OP:22.5 `ktfbbredo`)

## Overview

`OP:22.5` redoes a change to the file bitmap block (block 3) of a locally
managed tablespace. Oracle prints a banner, an operation name and one or two
detail lines:

```
ktfbbredo - File BitMap Block Redo:
Use Bits: 
Begin: 9280, Length: 1, Instance: 0 
```

All detail lines end with a trailing space. The sample set (5 blocks) only
contains `Use Bits`; the other operation names were established by patching the
operation byte.

## Payload layout

One 12-byte vector, the same shape as `OP:22.2` (file space header redo):

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `op` (u8) | operation name |
| 1 | `save` (u8) | no effect on the output |
| 2 | `instance` (u16) | `Instance:` of the `Use Bits` line |
| 4 | `begin` (u32) | `Begin:` / `New First:` |
| 8 | `length` (u32) | `Length:` / `New Free:` |
| 12 | — | vector ends |

## Operation table

| `op` | Name | Detail line |
|------|------|-------------|
| 0 | `Unknown OPcode: ` | — |
| 1 | `Free Bits: ` | `Begin: {}, Length: {} ` (no instance) |
| 2 | `Use Bits: ` | `Begin: {}, Length: {}, Instance: {} ` |
| 3 | `Copy Bits: ` | `New First: {}, New Free: {} ` plus fields outside this payload (see open items) |
| other | `Unknown OPcode: ` | — |

Unlike `OP:22.2`, there is no `Header Opcode:` line and op byte 0 is not a
"no pending operation" marker — it prints `Unknown OPcode: `.

## Probe evidence

Patches against the first sample's vector at `0x41C4`
(`02 000000 | 40 240000 | 01000000`, natural output `Begin: 9280, Length: 1,
Instance: 0`):

| Patch | Result |
|-------|--------|
| `op=1` (`0x41c4`) | `Free Bits: ` + `Begin: 9280, Length: 1 ` |
| `op=2` | unchanged (`Use Bits`) |
| `op=3` | `Copy Bits: ` + `Begin Byte: 1703945, No Bytes: 1170 ` + `New First: 9280, New Free: 1 ` + a 1170-byte hex dump |
| `op=0/4/5/8/10/ff` | `Unknown OPcode: ` |
| `save=1/2` (`0x41c5`) | no output change |
| `instance=0xbeef` (`0x41c6`) | `Instance: 48879` |
| `begin=0x11111111` (`0x41c8`) | `Begin:` / `New First:` = 286331153 |
| `length=0x22222222` (`0x41cc`) | `Length:` / `New Free:` = 572662306 |

`Begin Byte: 1703945` (`0x1a0009`) and `No Bytes: 1170` (`0x492`) do not come
from the 12-byte vector: patching `begin`/`length` leaves them unchanged, and
the values match the `ktudh` vector of the *next* redo record. When the op byte
of a `Use Bits` record is changed to 3, Oracle reads past the change payload;
a real `Copy Bits` record must carry an extended payload that is not present in
any sample.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `FileBitmapBlockRedo = 5 !layers::layer22::FileBitmapBlockRedo` |
| `crates/ointernal-parser/src/oracle/change/layers/layer22.rs` | `FileBitmapBlockOp`, `FileBitmapBlockRedoHead`, byte-slice wrapper |
| `crates/ointernal-dumper/src/dump/dump_layer22.rs` | `LocallyManagedTablespace` branch + printer |

## Verification

All 5 natural blocks match the Oracle dump line-for-line, including trailing
spaces: 4 in `TestOverwriteStatsHistogram` and 1 in `TestOverwriteCtas`. The
`Free Bits`/`Copy Bits`/`Unknown OPcode` outputs were checked against Oracle
dumps of patched files.

## Open items

- The extended `Copy Bits` payload (`Begin Byte`, `No Bytes`, then the copied
  bytes) has no natural sample. Our dumper prints only `Copy Bits:` and
  `New First: {}, New Free: {}` for op 3; a real record would additionally
  print the byte-range line and a hex dump.
- `OP:22.2` (`ktfbhredo`) and `OP:22.3` (`ktfbhundo`, embedded in `OP:5.24`)
  share the 12-byte save-area shape; the undo side still needs decoding
  (note 12).
