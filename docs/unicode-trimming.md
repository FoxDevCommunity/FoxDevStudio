# Unicode preservation in trimming

ALLTRIM, LTRIM, RTRIM and its TRIM alias preserve decoded Unicode text.
Previously they truncated every scalar to its low byte before trimming, so
non-Latin-1 text became unrelated characters, even when no spaces were present.
Trimming now slices the original string at character boundaries. Explicit
single-character parse arguments use the same character representation.

Default trimming still removes only ASCII space, not tabs, newlines or NBSP.
The U+0000–U+00FF byte-carrier representation is retained without re-encoding;
shared byte helpers and ALINES are unchanged.

This is a text preservation fix, not full VFP trimming parity. Existing parsing
arguments remain a character set; multi-character parsing strings, case flags
and distinct Varbinary zero-byte trimming need separate compatibility work.
VFP documents those behaviors in [ALLTRIM help](https://www.vfphelp.com/help/html/767f9fa5-5271-4fe2-baf8-332cf8d15fb5.htm).
Other character built-ins may still truncate Unicode through the byte helper.

Regression coverage includes eight script/character samples, single-sided and
custom-character trimming, low-byte collisions, all 256 byte-carrier values,
non-space whitespace and a compiled program running through the WASM bridge.
No native Windows VFP comparison was run for this change.
