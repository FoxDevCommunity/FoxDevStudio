# DBF text codepages

The DBF language-driver marker selects the decoding and encoding used for
character fields and text memo blocks, including form/class property memos.

Supported single-byte pages:

- Windows: 1250, 1251, 1252, 1253, 1254, 1255, 1256, 1257.
- Thai: 874.
- DOS: 437, 737, 850, 852, 857, 860, 861, 863, 865, 866.

The mapping references and their hashes are recorded in
`docs/codepage-mappings.md`. The test oracle covers every byte,
with undefined high-byte positions represented as U+FFFD. An unrepresentable
Unicode character is written as `?`, preserving the existing codec convention.
Undefined byte round trips are not guaranteed; this is a text codec, not a
lossless binary conversion. Binary memo blocks retain their original bytes.

## Limits

Unmarked, unknown and recognized-but-unimplemented pages retain the existing
CP1252 fallback. In particular, recognizing markers for East Asian or Macintosh
pages does not mean their codecs are implemented. The Mac Greek marker 0x98
reports 10006; it still has no dedicated codec.

This change does not implement full application locale semantics, collation,
RTL layout, keyboard input, font-specific legacy encodings or Unicode migration.
CPCURRENT/OEMTOANSI retain their existing fixed runtime assumptions. Plain text
source files and MEM files have separate encoding paths.

Some string built-ins still operate on low bytes of Unicode codepoints. For
example, the current ALLTRIM implementation corrupts Greek text even after a DBF
has decoded correctly. That requires a separate string-semantics fix. The DBF
write/reopen tests inspect fields directly to isolate the codec contract.

DBCS codecs must also handle character boundaries when writing fixed-width
fields; adding a decoder alone is insufficient.

## Tests

`cargo test -p foxvm --test dbf_codepages` covers canonical byte mappings,
character writes, memo reads, binary preservation, field width handling,
VM REPLACE/close/reopen for both character and memo fields, and CPCONVERT.
`tests/vfp/importForm.codepages.test.ts` imports synthetic form captions through
the built WASM module for all supported pages.
