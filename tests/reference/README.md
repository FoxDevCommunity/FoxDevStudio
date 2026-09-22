# The Visual FoxPro language reference

`vfp-language.tsv` is every command, function, preprocessor directive, system variable,
property, method and event the Visual FoxPro 9 SP2 reference lists: 1700 elements, one per row.
It is the yardstick this runtime is measured against, so that coverage is decided by the
language rather than by whichever sample happened to be opened last.

Refresh it with `node scripts/fetch-vfp-reference.mjs`. The pages come from vfphelp.com, the
community-maintained copy of the VFP 9 SP2 help file (Creative Commons BY 3.0,
github.com/VFPX/HelpFile).

`coverage.test.ts` asks the runtime what it does with each element and writes the answer to
`docs/language-coverage.md`. A command is classified by compiling a probe of it through the
real compiler and reading the diagnostic, so the answer is the compiler's, not a list kept by
hand. Regenerate with `REGEN_DOCS=1 npx vitest run tests/reference`.

A golden program in `crates/foxvm/tests/programs` may end with `* COVERS: NAME, NAME`, which is
what marks those elements as exercised rather than merely present. The header goes at the end of
the file because a golden expectation names the line an error came from.

## The base classes

`vfp-base-classes.tsv` is the second measurement: every base class a program can `CREATEOBJECT`,
with every member it answers to, the value each property starts out holding, and whether the
product refuses to have that property written to. It is what `src/shared/registry` and
`crates/foxvm/src/base_classes.tsv` are generated from, so the object model is the product's
rather than a list kept by hand.

Refresh it with `node scripts/vfp-base-classes.mjs`, then regenerate the two tables with
`node scripts/gen-base-classes.mjs`. Both need Visual FoxPro 9 and take a few minutes.

Two things in it are the measuring machine's rather than the product's, and a re-measurement
elsewhere will differ in them: the selected-item colours follow the Windows theme, and `Text` on
a text box is as wide as the control was drawn.
