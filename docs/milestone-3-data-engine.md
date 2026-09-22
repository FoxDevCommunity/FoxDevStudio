# Milestone 3: Data engine design

## Why this needs its own design

Milestone 2 put the compiler and VM in WebAssembly, and that was the right call: one build for every
platform, no native toolchain for contributors, tests that run under jsdom, and a runtime player that
is plain Electron with no compiled dependency to rebuild.

The data engine is the first part that WebAssembly is actively wrong for.

- **wasm32 is 32-bit.** The whole module shares one linear address space with a hard ceiling near 4 GB,
  and in practice much less. A table larger than a gigabyte cannot live there alongside the runtime.
- **WebAssembly cannot open a file.** It has no file descriptors and no seeking; every byte has to be
  handed to it by the host.
- The current reader takes the whole file as a byte slice. That is correct for a 2 KB `.scx` and wrong
  for a 4 GB table.

Visual FoxPro stops at 2 GB per table because it uses signed 32-bit file offsets internally. Doing
better than that is a deliberate goal here, not an accident of the design.

## The split

**The host owns bytes. The VM owns meaning.**

| Concern | Where | Why |
|---|---|---|
| Open, seek, read, write, lock, close | Electron main process | 64-bit offsets, real file handles, memory mapping |
| Header and field layout | Rust (`dbf::read_header`) | already written and tested |
| Record decoding, type coercion | Rust, inside the VM | one implementation, no duplicate parser in TypeScript |
| Cursor state, filters, relations, SQL | Rust, inside the VM | it is language semantics, not I/O |

The VM never sees a path and never sees a file size. It asks for *a page of record bytes at record
number N*, and the host answers with those bytes. Everything the VM does with them, it already knows
how to do.

The crossing therefore carries **raw record bytes in batches**, not decoded values one at a time.
That is what keeps it fast: a 256-record page of a 130-byte table is one 33 KB copy, not 256 bridge
calls returning JavaScript objects.

## Protocol

New host requests, alongside the existing `SetProp`, `MessageBox` and the rest. Everything is
64-bit: record numbers and byte offsets are `f64` on the wire, which is exact to 2^53 records.

```rust
pub enum DataRequest {
    /// USE: resolve the table, its memo file and any structural index. Answers with a handle,
    /// the raw DBF header bytes (the VM parses them), and the record count.
    Open { path: String, alias: String, exclusive: bool, again: bool },
    /// Read `count` records starting at `first` (1-based), as one contiguous block of bytes.
    ReadRecords { handle: u32, first: f64, count: u32 },
    /// A memo block, by the pointer stored in the record.
    ReadMemo { handle: u32, block: f64 },
    /// REPLACE and friends: write one record back in place.
    WriteRecord { handle: u32, recno: f64, bytes: Vec<u8> },
    /// APPEND: add a record and report its number.
    AppendRecord { handle: u32, bytes: Vec<u8> },
    Close { handle: u32 },
}
```

The host implementation is small because it does nothing clever: `open`, `read` at an offset,
`write` at an offset. The offset arithmetic (`header_length + (recno - 1) * record_length`) happens
in the VM, where the layout is already known.

## Why not ship a native Rust library

It is the obvious next thought, and the answer is: not yet, but the design must not preclude it.

**Against, today.** A native addon has to be compiled per operating system, per architecture, *and*
per Electron ABI version, then rebuilt whenever Electron is upgraded. It turns a one-command build
into a prebuild matrix and a CI story. It also breaks the property that the runtime player is plain
Electron with nothing to compile, which is what makes Build Executable a directory copy.

**For, later.** Two things would justify it: building and maintaining `.cdx` index files over
millions of rows, and SQL joins where the work is genuinely CPU-bound rather than I/O-bound. Both are
measurable, and neither is on the critical path for correctness.

**So the seam is the deliverable.** Because the host side is *only* seek-and-read behind
`DataRequest`, replacing it with a native addon later means implementing the same handful of methods
in Rust and changing nothing in the VM, the language, or the tests. If instead the host had been
handed "parse this table and give me rows", swapping implementations would mean rewriting the parser.

The decision to make now is therefore not native-versus-wasm. It is: **keep every byte-level operation
behind a request the host answers, and never assume a file fits in memory.**

## Beyond 2 GB

Concretely, what lifts the VFP limit:

- **Record offsets are computed in 64-bit** (`f64` on the wire, `u64` in the host), never `u32`.
- **No whole-file reads.** `read_table(bytes)` stays for the importer, where files are kilobytes, and
  the data engine never calls it.
- **Memo blocks are addressed by 64-bit offset.** VFP stores a 32-bit block *number*; multiplied by
  the block size that already exceeds 2 GB for block sizes above one, so the format allows it and only
  VFP's internal arithmetic did not.
- **The page cache is bounded** and holds decoded records for the current work area only, so memory is
  a function of the working set, not the table.

A table larger than 2 GB will not open in Visual FoxPro. It should open here.

Measured on Node 24 before committing to this: a sparse file was written and read back at a
5 GB offset, and at the offset record 40,000,000 of a 130-byte table occupies (5,200,001,062),
both correct. Offsets stay exact as doubles well past that, so the wire format needs no bigint.
The remaining ceilings are the DBF format's own 32-bit fields: 4.29 billion records from the
header count, and memo data capped at the 32-bit block number times the block size, which is
only 4 GB when a file uses a block size of 1 as the real samples do.

## Phasing

1. **Cursor layer.** `USE`, `SELECT`, work areas, `GO`, `SKIP`, `RECNO/RECCOUNT/EOF/BOF`, field
   reads and `REPLACE`. Read-only first, then writes.
2. **Navigation and filters.** `LOCATE/CONTINUE`, `SCAN...ENDSCAN`, `SET FILTER`, `SET DELETED`,
   `DELETE/RECALL`, `APPEND BLANK`.
3. **Indexes.** Read `.cdx` for `SEEK` and `SET ORDER`; build and maintain them after that.
4. **SQL.** `SELECT-SQL` over cursors, then `INSERT/UPDATE/DELETE`. Joins on top of the index layer.
5. **Binding.** `ControlSource` and `RowSource` on the existing controls, and a Grid that shows real
   rows. This is where the data engine becomes visible in the IDE.

The seam in step 1 is what the rest depends on; the ordering after that can move.

## What this changes in Milestone 2's code

Nothing structural, which is the point. `HostRequest` already exists and already has a reserved
`Data` variant. The scheduler already performs requests with the VM off the stack, so a read that
touches the disk parks the fiber exactly as a dialog does. The built-in functions that currently
report "the data engine arrives in a later milestone" become real implementations behind the same
names.
