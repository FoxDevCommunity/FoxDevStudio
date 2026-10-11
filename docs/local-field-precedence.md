# Fields and same-named local variables

Bare variable expressions now resolve a field in the selected work area before
a same-named LOCAL or LPARAMETERS slot. If there is no matching field, the local
slot still wins over private/public variables. Explicit m.name remains a direct
memory-variable read, and assignment targets still write memory variables.

Previously a declared local compiled directly to LoadLocal even for an
unqualified expression. A pattern such as LOCAL m.item_id followed by
m.item_id = item_id read the uninitialized local (.F.) instead of the numeric
field. Inserting that value into an integer column then raised error 9.

The compiler now emits LoadName for bare expression reads; the VM checks fields,
then the current function's local slots, then the existing dynamic-name lookup.
Internal temporary slots and explicit m.name still use LoadLocal. No bytecode
layout changed. Rebuild compiled projects/bundles to obtain the corrected reads.

Reference: [VFP Accessing Variables](https://www.vfphelp.com/help/html/4b4bccf2-fa78-4377-b838-a94863efb6b2.htm).

The synthetic regression failed with error 9 before the fix. Six focused tests
cover insertion, explicit m., local fallback, assignments, parameters, public
shadowing and EVALUATE. The full VM suite (680 tests including doctests), 16
WASM bridge/sample tests, WASM build, typecheck, lint and app build pass on macOS
ARM64. Native Windows VFP was not run. The alternate m-> spelling remains an
existing parser limitation outside this fix.
