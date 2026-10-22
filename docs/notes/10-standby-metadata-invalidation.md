# Standby Metadata Cache Invalidation (OP:17.28)

## Overview

`OP:17.28` (layer 17, `RecoveryManagement`) is the `STANDBY METADATA CACHE
INVALIDATION` marker: after a commit on a system table Oracle logs the RAC /
standby metadata caches that must be invalidated. Oracle prints a banner plus
two sections:

```
STANDBY METADATA CACHE INVALIDATION
KQR INFO:
[2c017bc3, 2, 0]
[715b2356, 8, 16038]
KGL INFO:
[504592d3.17ca2979.493abc5e.ef89d5f0, 1]
[bd3027fc.36d826ca.127b6aed.af31079a, 53]
```

Both headers are always printed, even when the corresponding list is empty. The
change header marks it as `MEDIA RECOVERY MARKER` (`TYP:6`, all other header
fields zero).

Samples live in `TestOverwriteStatsHistogram` (10), `TestOverwriteStatsHistogram2`
(6) and `TestOverwriteCtas` (3); they are logged right after a `ktucm` change in
the same record. Most records carry only one or two entries; one record in
`TestOverwriteStatsHistogram` carries 9 KQR and 12 KGL entries, and one vector
crosses a block boundary.

## Payload layout

Two vectors, one per section:

| Vector | Content | Entry size |
|--------|---------|------------|
| 0 | `KQR INFO:` entries | 8 bytes |
| 1 | `KGL INFO:` entries | 20 bytes |

Either vector can be empty (size 0) while the other carries data.

### KQR entry (8 bytes)

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `id` (u32) | first number, lowercase hex, no zero padding |
| 4 | `value` (u16) | third number |
| 6 | `kind` (u16) | second number |

Oracle prints `[{id:x}, {kind}, {value}]`; the two halves of the second word are
printed high half first. Probes on record #54's vector (`0x4B04`):

```
id=0xABC       word1=0x000B0005 -> [abc, 11, 5]
id=0x1234567   word1=0x0001000F -> [1234567, 1, 15]
```

### KGL entry (20 bytes)

| Offset | Field | Output |
|--------|-------|--------|
| 0 | `id[4]` (4 × u32) | four lowercase hex words joined by `.` |
| 16 | `count` (u32) | last number = **low byte only** |

Oracle prints `[{id0:x}.{id1:x}.{id2:x}.{id3:x}, {count & 0xff}]`. Each word is
the little-endian value of the four bytes, so the raw bytes `21 49 5f a3` are
printed as `a35f4921`. Leading zeros are dropped (`06 0a 15 91` → `60a1591`).
Words 1-3 of the count dword are `#{AMND}`: patching `0x3350` from `04000000`
to `0F112233` changed the count from 4 to 15 and nothing else, while the
observed records carry garbage there (`0449b867`, `04060080`) without affecting
the output.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `StandbyMetadataCacheInvalidation = 28 !layers::layer17::StandbyMetadataCacheInvalidation` |
| `crates/ointernal-parser/src/oracle/change/layers/layer17.rs` | `KqrInfo`, `KglInfo`, wrapper |
| `crates/ointernal-dumper/src/dump/dump_layer17.rs` | `RecoveryManagement` dispatch + both sections |

Empty vectors are skipped before `ref_from_bytes`, which would otherwise reject
a zero-length slice for an alignment mismatch.

## Verification

The 19 payload sections in the three archives (78 + 36 + 16 lines) are
identical to the Oracle dump. The `CHANGE #…` header line still uses the
generic format, so `MEDIA RECOVERY MARKER` is not reproduced there.
