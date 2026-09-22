# Covering the language reference

The target is every element of the Visual FoxPro 9 reference: 1652 commands, functions,
directives, system variables, properties, methods and events. `docs/language-coverage.md` is
generated from the runtime itself and says where we are; this says how we get to the end and in
what order.

## What "covered" means

An element is covered when a program that uses it behaves as it does in Visual FoxPro, and a
test says so. Three things short of that are not coverage, and the map names each separately:
accepted and ignored, refused by name, and unknown.

A few elements cannot behave as they do in VFP, because what they act on does not exist here:
the character screen `@ ... SAY` draws on, the printer `SET DEVICE TO PRINTER` selects, the
design surfaces `MODIFY REPORT` opens. Those are covered when they are refused by name with a
reason, and the map will keep them in a category of their own rather than counting them as
gaps. Nothing else qualifies for it: "hard" is not a reason.

## Hard rule: every argument the reference gives, measured

An element is not covered by its name. It is covered by each of the argument forms its reference
page documents, each one measured against the product.

`SET(cSetting [, nSecondSetting])` is why the rule is written down. We implemented the second
argument for the three settings whose second form a golden happened to use, and let every other
name fall through to the switch. `SET()` counted as covered, and it was not: the samples' own
`SET("HELP", 1)` came back `"ON"` instead of the help file, a form's `RestoreHelp` then ran
`EVAL("ON")`, and the user met "Variable 'ON' is not found" by opening a sample. Asking the
product for all three answers of all ninety settings took one probe and found seventeen with a
second half we did not have. The same shape of miss had already cost us `MODIFY <kind>`, where
each kind supplies its own extension and only one kind was measured.

So, when covering an element:

- read its page and list every optional argument, every clause, and every combination that
  changes the answer - not the one form the sample in hand uses,
- ask the product for all of them in one probe, sweeping the whole family where there is one
  (all settings, all `MODIFY` kinds, all `SYS()` numbers) rather than the single name at hand,
- put each form in the golden, so the claim in `COVERS:` is backed by every form and not by one.

A form the product cannot be asked about goes in `not-measured.txt` with the reason, the same
as any other unmeasurable element.

## Method

Each wave is implemented against the reference page for the element, not from memory. The page
gives the syntax, the arguments, the return value and the edge cases; where behaviour depends on
a setting (`SET MEMOWIDTH` for the memo functions, `SET EXACT` for comparison) the setting is
implemented with it. Every wave ends with:

- the elements' behaviour in a golden program, with a `COVERS:` header naming them,
- `docs/language-coverage.md` regenerated, so the count moves,
- `npm run check` green.

## Waves

1. **Pure functions.** Maths, bit, string, conversion, memo-line and date functions that need
   nothing outside the VM. About 120 functions, no host, no data engine.
2. **Files.** The `FOPEN`/`FREAD`/`FWRITE` family, `ADIR`, `DIRECTORY`, `FULLPATH`,
   `COPY FILE`, `RENAME`, `MD`/`RD`, and the file commands that go with them.
3. **Indexes.** `.cdx` reading and writing, `INDEX`, `REINDEX`, `SET ORDER`, `SEEK`, `SEEK()`,
   `TAG()`, `KEY()`, `SET RELATION`, `SET FILTER`. The largest hole in the data engine, and what
   a form's `Order` needs.
4. **Tables.** `COPY TO`, `COPY STRUCTURE`, `APPEND FROM`, `SCATTER`/`GATHER`, `PACK`, `SORT`,
   `CALCULATE`, `SUM`, `AVERAGE`, `COUNT`, `TOTAL`, `ALTER TABLE`, `DROP TABLE`, and record
   locking.
5. **Buffering and transactions.** `TABLEUPDATE`, `TABLEREVERT`, `CURSORSETPROP`,
   `GETFLDSTATE`, `GETNEXTMODIFIED`, `BEGIN`/`END`/`ROLLBACK TRANSACTION`.
6. **Databases.** `.dbc` as a container: `OPEN DATABASE`, `SET DATABASE`, `DBGETPROP`,
   `DBSETPROP`, views, relations, triggers, `VALIDATE DATABASE`.
7. **The object model.** The 359 properties, 106 events and 88 methods the reference lists that
   the registries do not, control by control.
8. **Screens and menus.** `@ ... SAY`/`GET`, `DEFINE WINDOW`, `DEFINE POPUP`, `BROWSE`, and the
   window and menu functions, drawn on the runtime desktop rather than on a character screen.
9. **Reports.** `.frx` as a document, `REPORT FORM`, `LABEL`, the ReportListener.
10. **The rest.** SQL pass-through, XML, DDE, the remaining COM surface.
11. ~~After the last wave.~~ Dropped. The review it described is now part of
    every unit of the parity plan rather than a wave of its own, and editor autocompletion is
    cut: it is polish, and there is a product to finish first.


The order is by what unblocks the most: a wave that needs nothing comes before one that needs
the data engine, and the object model comes after the language it is driven by.

### Known for the review

- **A string is a string of bytes.** The character functions work on the UTF-8 bytes of the
  text, while `CHR()`, `ASC()` and everything that reads a file treat one character as one
  byte. They agree for ASCII and disagree above it: `LEN(BINTOC(258, 2))` answers 3 where
  Visual FoxPro answers 2. Deciding on one convention and holding the whole runtime to it is
  the review's to do, because it touches every function in `builtins/string.rs`.

## After the map

The map is not the product. What is left between a covered language reference and Visual FoxPro 9
is sixteen systems - the formats we read but cannot write, the debugger, the designers, the
resolver that makes other people's code run - and they are planned in
[parity-plan.md](parity-plan.md). It is not a wave per element; that method finished its job
here.
