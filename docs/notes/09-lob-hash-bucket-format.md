# LOB Hash Bucket Block Format (OP:13.43)

## Overview

`OP:13.43` is `KTSL - Format LOB Hash Bucket Block`: the redo record that
formats a LOB hash bucket block as a free-space block. Oracle prints two lines:

```
KTSL - Format LOB Hash Bucket Block (CFS)
objd: 78053 inc: 0 inst:1 flag: 0x5 range: 2k-32k
```

`TestOverwriteStatsHistogram` contains 24 of them, emitted while the LOB's hash
bucket blocks are formatted: two `UFS` blocks, then 8/4/2/2/2/2/2 `CFS` blocks
for the successive size classes.

## Payload layout

One 48-byte vector (12 words). Only five words are used by the dump:

| Offset | Word | Field | Output |
|--------|------|-------|--------|
| 0 | w0 | size class | `range:` (CFS only) |
| 4 | w1 | instance | `inst:` |
| 8..16 | w2-w4 | `#{AMND}` | — |
| 20 | w5 | flags | `(CFS)` / `(UFS)` / `(Invalid)` and `flag:` |
| 24..36 | w6-w9 | `#{AMND}` | — |
| 40 | w10 | object id | `objd:` |
| 44 | w11 | increment | `inc:` |

The words at 8, 12, 16, 24, 28, 32 and 36 were each patched with `0xDEADBEEF`
on a CFS sample (range `2k-32k`) without any change to the Oracle output; word
11 was 0 in every sample (`inc: 0`).

### Flag classification (bit 0 / bit 1)

| `flag` | Banner | Second line |
|--------|--------|-------------|
| bit 0 set (5, 3, 7, ...) | `(CFS)` | `... flag: 0x.. range: <class>` |
| bit 0 clear, bit 1 set (6, 2) | `(UFS)` | `... flag: 0x..  ` (two trailing spaces) |
| neither (0, 4) | `(Invalid)` | `... flag: 0x..  ` (two trailing spaces) |

Byte modification at `0x55C4` (word 5 of the first 13.43 vector):

```
flag=5 -> (CFS)     objd: 78053 inc: 0 inst:1 flag: 0x5 range: 2k-32k
flag=7 -> (CFS)     objd: 4660 inc: 34952 inst:153 flag: 0x7 range: Invalid
flag=3 -> (CFS)     objd: 78053 inc: 0 inst:1 flag: 0x3 range: 2k-32k
flag=2 -> (UFS)     objd: 78053 inc: 0 inst:1 flag: 0x2
flag=4 -> (Invalid) objd: 78053 inc: 0 inst:1 flag: 0x4
flag=0 -> (Invalid) objd: 78053 inc: 0 inst:1 flag: 0x0
```

(The `flag=7` probe also set `w0=0xEE`, `w1=0x99`, `w10=0x1234`, `w11=0x8888`,
which is how `range: Invalid`, `inst:153`, `objd: 4660` and `inc: 34952` were
identified.)

Independent single-word probes on the same vector confirm each field in
isolation: `w0=3` → `range: 128k-256k`; `w1=7` → `inst:7`; `w10=0x1111` →
`objd: 4369` (the change header `OBJ` stayed 78053); `w11=0x2222` →
`inc: 8738`; `flag=0x1` → `(CFS)`; `flag=0xa` / `0xe` → `(UFS)`. Setting
`w0=7` with `flag=6` keeps `(UFS)` and prints no `range`, so the size class is
only decoded for CFS.

### Size class (`range`)

`range` selects the LOB free-space size class printed after the flag:

| `range` | Label |
|---------|-------|
| 0 | `2k-32k` |
| 1 | `32k-64k` |
| 2 | `64k-128k` |
| 3 | `128k-256k` |
| 4 | `256k-512k` |
| 5 | `512k-1m` |
| 6 | `1m-64m` |
| other | `Invalid` |

The 24 observed records walk these classes in order: class 0 is logged eight
times, class 1 four times and each of classes 2-6 twice.

## Implementation

| File | Change |
|------|--------|
| `crates/ointernal-parser/src/oracle/change/opcode.rs` | `FormatLobHashBucketBlock = 43 !layers::layer13::FormatLobHashBucketBlock` |
| `crates/ointernal-parser/src/oracle/change/layers/layer13.rs` | `FormatLobHashBucketBlockHead` (48 bytes) + wrapper |
| `crates/ointernal-dumper/src/dump/dump_layer13.rs` | banner + flag/range line |

## Verification

All 24 `OP:13.43` blocks in `TestOverwriteStatsHistogram` match the Oracle dump
line-for-line, including the two trailing spaces of the non-CFS line.

## Open items

CFS changes carry a second 48-byte vector that Oracle never prints. It is
constant in the low words and holds apparent stack pointers
(`0x00007fff760a9828`) in words 6 and 8, with word 10 varying between 2, 4 and
8, so it looks like a printed-nowhere descriptor. The parser ignores it.

