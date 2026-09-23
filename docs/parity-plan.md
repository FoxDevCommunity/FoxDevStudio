# Feature parity with Visual FoxPro 9 - the plan of record

Waves 1-10 covered the language reference: every command, function, property, event, method,
object, operator, directive and system variable it names is known to the runtime, and nothing is
unknown. That was the right way to find the shape of the product. It is the wrong way to finish
it - element by element, a golden program and a document regeneration each, is whack-a-mole, and
most of what is left is not element-shaped anyway. There is no designer entry in the language
reference.

This document replaces that method. The remaining work is **sixteen systems**. Each is a thing
that either exists or does not; each is built once, and the reference elements that belong to it
fall out of it in a batch rather than one at a time.

## How the work goes now

Six rules, all of them aimed at the same thing - stop paying per element.

1. **A unit is a system, not a list.** "Every display setting goes through one formatter" is a
   unit. "Implement `SET CURRENCY`" is not; it is a line in a table inside that unit.
2. **Table-driven wherever the reference is a table.** The settings, the colour schemes, the
   error codes, the field types, the report band types: data, read once, not thirty functions.
   N elements must cost the same as one.
3. **One reference read per system.** The reference pages for a system are read together, at the
   start of it, and what they say goes into the design. Not one page, one commit, one page.
4. **One golden program per system**, covering its whole surface, with the `COVERS:` header
   naming everything it reached. One `npm run check`. One document regeneration. One commit.
5. **The review is part of the unit.** Each unit re-reads the elements it owns that were already
   marked "runs" and fixes what runs wrongly. There is no separate adversarial-review wave; a
   system is not done while part of its surface lies.
6. **Leverage crates, never hand-roll.** Images, printing, DBF, COM, DDE, git - each has an
   established crate or an Electron API. The hard rule stands.

Cut, explicitly: editor autocompletion with per-element information. It is polish and it is not
parity. It comes back only if everything below is done.

## The sixteen systems

Each has: what it is, what it clears, and what "done" means.

---

### 1. Documents of our own for what has none yet

**Decided: the Visual FoxPro formats are import-only.** `.scx`, `.vcx`, `.mnx` and `.pjx` are
read and converted; nothing is written back. A form authored here cannot be opened in Visual
FoxPro 9 again, and that is the trade: FoxDev is where a codebase lands, not a second editor
working the same files beside VFP.

What that buys is most of a system's worth of work not done - matching five DBF-and-memo record
layouts byte for byte, and keeping them matched - and formats that diff, merge and review like
source. What it costs is one promise, and the one thing that would take it back is a team
migrating a file at a time with some developers still in VFP 9. If that is ever the ask, the
answer is a **Save As Visual FoxPro** export, lossy and warned about, not a round-trip
guarantee: far less work than writing every format faithfully, and it can arrive late.

The three places import-only was checked, and what each costs:

- **Classes.** `.vcx` already imports to form-shaped documents, so a class library authored here
  is ours; what a running program needs is to *read* one, which is the importer.
  `SET CLASSLIB TO lib.vcx` and `NEWOBJECT('x', 'lib.vcx')` keep working. Cost: none.
- **Reports.** `.frx` is read at run time by the VM itself (`crates/foxvm/src/report.rs`), not
  imported. The designer authors a document of ours and `REPORT FORM` reads both. Cost: none -
  a second reader, not a writer.
- **A program that treats these files as tables.** `USE myform.scx` works: they are DBFs, and
  the engine reads any DBF. Writes to them go through as well; what does not happen is the
  designer noticing. That is the whole of the loss, it is narrow, and builders that do it are
  the only thing in it.

So the work here is not a writer. It is the two document formats that do not exist yet, beside
the form, menu and project ones that do:

- **A class library document.** `.fxc` is already a project item kind with no format behind it.
  It is the form schema plus a parent class and which values are inherited.
- **A report and label document.** Bands, groups, expressions, the data environment - the last
  thing in the product still read only in its Visual FoxPro shape.

**Done when.** Every `.scx`, `.vcx`, `.frx` and `.mnx` in the VFP 9 samples tree imports with
no loss the baseline does not already name, the class and report designers open and save
documents of ours, and `REPORT FORM` runs either shape.

### 2. Values: strings, numbers, dates, money, code pages

**What.** One `Settings`-driven formatter and comparator that every display, conversion and
comparison in the runtime goes through, and one decision about what a character is.

**Clears.** The byte-string inconsistency (`LEN(BINTOC(258,2))` answers 3 where VFP answers 2 -
this touches every function in `builtins/string.rs`); exact Currency as a scaled i64 rather than
`f64`; `SET CURRENCY POINT SEPARATOR HOURS SECONDS FIXED DECIMALS NULLDISPLAY FDOW FWEEK MARK TO
STRICTDATE SYSFORMATS DATE CENTURY ANSI EXACT COLLATE NOCPTRANS CPCOMPILE CPDIALOG`;
`CPCONVERT`, `STRCONV`, `OEMTOANSI`, the DBF code-page byte.

**Done when.** One golden program prints the same bytes as real VFP 9 for every combination in
the settings table, run headlessly against `vfp9.exe` the way the `.mem` format was verified.

---

### 3. Names: where a routine, a class and a file come from

**What.** One resolver for "given a name, what does it mean here" - the search order VFP uses
across the current program, procedure files, class libraries, the path, and the resource file.

**Clears.** `SET PROCEDURE`, `SET CLASSLIB`, `SET PATH`, `SET DEFAULT`,
`SET FULLPATH`, `SET RESOURCE`, `LOCFILE`, `EXECSCRIPT`, `SET UDFPARMS`, `SET COMPATIBLE`,
`SET ENGINEBEHAVIOR`, and the remaining macro-expansion corners.

**Why early.** A great deal of real VFP is written as `SET PROCEDURE TO lib` and then bare calls.
Without it, other people's code does not run at all, which no amount of covered elements fixes.

---

### 4. The data engine's second half

**What.** The parts of the engine that were left as settings rather than behaviour, plus the
optimiser.

**Clears.** `SET FIELDS`, `SET INDEX`, `SET NULL`, `SET SKIP`, `SET CARRY`, `SET REFRESH`,
`SET TABLEVALIDATE`, `SET DATASESSION`, `SET AUTOINCERROR`, `SET RELATION OFF`, `SET OPTIMIZE`
and Rushmore itself - queries and `LOCATE`/`SCAN` narrowing through an index instead of reading
every record. Multi-process locking (`FLOCK`, `RLOCK`, `SET REPROCESS`, `SET MULTILOCKS`) tested
with two processes on one table, not one.

**Done when.** A million-record table answers an indexed `SELECT` in the time VFP takes, and the
locking tests run two real processes.

---

### 5. The debugger

**What.** Breakpoints, stepping, Trace, Locals, Watch, Call Stack, Output; `SUSPEND`, `RESUME`,
`SET STEP`, `SET ECHO`, `SET DEBUG`, `SET DEBUGOUT`, `DEBUGOUT`, `ASSERT`, `SET ASSERTS`; the
Coverage Profiler over `SET COVERAGE`.

**Why it is cheap and must not be late.** The VM was built for it and has been waiting:
`Instr::Stmt(line)` marks every statement, `vm.rs` has a `breakpoints` field, the M2 plan
reserved `HostRequest::Break`, and a suspended program is just a parked fiber, so the IDE stays
live while it is stopped. Watch expressions compile through the `compile_expression` that
`EVALUATE()` already uses. Almost all of it is UI over machinery that exists.

**Done when.** A breakpoint in a form method stops there, Locals shows the frame, a watch
expression updates as you step, and the call stack walks back through `DO FORM`.

---

### 6. The class designer and the Class Browser

**What.** A visual class designer over `.vcx`, with subclassing: inherited property values shown
as inherited, overrides marked, `ParentClass` real. The Class Browser over a library.

**Clears.** `CREATE CLASS`, `CREATE CLASSLIB`, `ADD CLASS`, `REMOVE CLASS`, `MODIFY CLASS`,
`SaveAsClass`, `CloneObject`, `ResetToDefault`.

**Note.** The form designer already edits a tree of objects with properties and method source.
This is that designer over a class record plus inheritance, not a new thing.

---

### 7. Reports, labels and the printer

**What.** A report and label designer (bands, groups, expressions, the data environment, Quick
Report from a structure) and a real print pipeline.

**Clears.** `CREATE REPORT`, `CREATE REPORT - Quick Report`, `CREATE LABEL`, `MODIFY REPORT`,
`MODIFY LABEL`, `LABEL`, `DEFINE BOX`, `SET DEVICE TO PRINTER`, `SET PRINTER`, `SET MARGIN`,
`SET PDSETUP`, `@ ... SAY` to a page, `PRTINFO`, `SET PALETTE` for output.

**Mechanism.** Electron's `printToPDF` and `print`. The ReportListener object model
(`OutputPage`, `Render`, `GetPageHeight`, the page counts) is already in place and drives it.

---

### 8. The IDE's remaining windows

**What.** Project Manager as a real window (the `Project`/`File` objects and the `Query*` hooks
already exist behind it), Data Session, Document View, Task List, Query and View designers,
Database designer, Table designer (`CREATE`/`MODIFY STRUCTURE`), a Browse window worth the name.

**Clears.** `CREATE`, `CREATE FROM`, `CREATE QUERY`, `CREATE VIEW`, `CREATE TRIGGER`,
`MODIFY DATABASE`, and the last of the "opens a designer" commands.

---

### 9. Windows, docking and toolbars

**What.** The IDE's own window system: dockable panes, tear-off toolbars, `_SCREEN` as a real
MDI parent, window state persisted in the resource file.

**Clears.** `DOCK`, `Dockable`, `Docked`, `DockPosition`, `GetDockState`, `AfterDock`,
`BeforeDock`, `UnDock`, `SET RESOURCE`'s window half, the `WindowType`/`Desktop`/`MDIForm`
properties acting for real.

---

### 10. Building and shipping

**What.** `BUILD APP`, `BUILD EXE`, `BUILD PROJECT`, `COMPILE`, and a decision on `BUILD DLL` /
`BUILD MTDLL` - an in-process COM server is what they make, and `crates/foxole` is where one
would register; if out-of-process is the answer, say so and refuse the multithreaded one with a
reason.

**Already there.** `src/main/services/buildService.ts` and `shared/runtime/bundle.ts` do the
work; the commands do not reach them. Also here: an end-user installer and a slim runtime-only
player build.

---

### 11. Interop: COM, OLE and reflection

**What.** COM in depth - `GETINTERFACE`, `EVENTHANDLER` and event sinks, `COMCLASSINFO`,
`COMARRAY`, `COMRETURNERROR`; OLE document embedding, which is what `DoVerb`, `APPEND GENERAL`,
`MODIFY GENERAL`, the OleControl/OleBoundControl and `AutoVerbMenu` all need; reflection -
`AMEMBERS`, `GETPEM`, `PEMSTATUS` in full, from the registries that already know the answers.

---

### 12. DDE

**What.** The twelve `DDE*` functions, or a documented permanent refusal. Windows still
implements DDE and a Win32 binding is small and self-contained; what is not acceptable is
leaving twelve elements refused with no decision behind it.

---

### 13. Graphics, colour and geometry

**What.** `LOADPICTURE`, `SAVEPICTURE` (an image crate, not hand-rolled decoding),
`OBJTOCLIENT`, and the colour system: `RGBSCHEME`, `SCHEME`, `CREATE COLOR SET`, `SET COLOR TO`,
`SET COLOR OF`, `SET COLOR OF SCHEME`, `SET INTENSITY`, `SET BORDER`. The colour side is a
closed table - 24 schemes of 10 pairs in a `.ftm` file - so it is cheap and clears seven
elements at once.

---

### 14. The keyboard

**What.** Keyboard macros and function keys: `PLAY MACRO`, `SAVE MACROS`, `RESTORE MACROS`,
`CLEAR MACROS`, `SET FUNCTION`, `SET MACKEY`, `.fky` files, `ON KEY LABEL` reaching them,
`SET TYPEAHEAD`, `SET KEY`, `SET CONFIRM`, `SET BELL`, `KEYBOARD`, `INKEY`, `LASTKEY`,
`CHRSAW`. A recorder in the IDE, a player in the runtime. The `.fky` layout is read against real
VFP the way `.mem` was.

---

### 15. Source control

**What.** An SCC seam - `interface SourceControlProvider` - and one implementation backed by
git, so `AddToSCC`, `CheckIn`, `CheckOut`, `GetLatestVersion`, `RemoveFromSCC` and
`UndoCheckOut` do what they say, `SCCProvider`/`SCCStatus` report it, and the Project Manager
shows status per file.

VFP's model is file-level check-out with exclusive locks and maps onto git badly. The seam exists
so that mapping is one decision in one place rather than six. Not building a Visual SourceSafe
client is fine; having no provider at all is not.

---

### 16. Help

**What.** `HELP`, `SET HELP`, `SET TOPIC`, `SET TOPIC ID`, `HelpContextID`, `WhatsThisHelp`,
`ShowWhatsThis`, `WhatsThisMode`. A viewer over the VFP `.chm`, or better, over
`docs/language-reference.md` - the same source the coverage is measured against, which makes F1
help in the editor free.

---

## Never built, and the map should say so

Three refusals that are answers, not gaps, and belong in a category of their own so they stop
counting against us:

- **`CALL`, `LOAD` and `SET LIBRARY` of a `.bin`** - 16-bit FoxPro 2.x binary overlays. There
  is nothing to load them into. `DECLARE - DLL` is the living replacement and works. This is
  about the `.bin` form only: `SET LIBRARY TO` a `.fll` loads a real Visual FoxPro library and
  its functions become part of the language, which is what every serious application needs.
- **`MODIFY SCREEN`, `CREATE SCREEN`** - the FoxPro 2.x screen designer. VFP 9 itself converts a
  `.scx` screen into a form; so do we.
- **`ASSIST`** - the FoxPro 2.6 Catalog Manager, which VFP 9 ships as a stub.

## Where to pick up

**The yardstick is the corpus, not the element count.** `tests/vfp/samples-known.txt` holds every
issue left against the 247 files Visual FoxPro ships in `Samples`. Today: **76 issues across 44
files, so 203 of 247 import and compile clean - 82%.** The element map says 96% and means much
less: it counts names known to the runtime, not whether a form opens. To reach 95% of the corpus
about 32 more files have to come clean.

What is left, largest first, and the two that would get there:

1. **17 `expected end of line, found X`** and about a dozen other one-line parser gaps -
   assignment targets, the WITH shorthand, aliases, member names. Each is a real Visual FoxPro
   construct, each is individually small, and the test names the file and the line for every one.
   Spread across roughly fifteen files: the cheapest route to 95%.
2. **14 data environment relations.** A form keeps its cursors but not the relations between
   them, so a one-to-many form opens unlinked. One system, about eight files.
3. Six `APPEND GENERAL` (OLE embedding, M5), three shortcut menus, three `RETURN TO`, and ones
   and twos after that.

**Two faults reported from the IDE are open** and written up under "Open, reported from the IDE"
above: a class method reaching the compiler with more text than the method has, and a form's data
environment cursor not being opened (the `SOLUTIONS` error). Both are something already built
getting it wrong.

**How to measure anything Visual FoxPro does.** It is installed and runs headlessly:

```
"C:/Program Files (x86)/Microsoft Visual FoxPro 9/vfp9.exe" C:\path\to\probe.prg
```

The program must start with `ON ERROR ?? ""` and end with `QUIT`, or an error opens a modal
dialog and it hangs; write results with `STRTOFILE(text, "out.txt")` to a relative path and run
it from that directory; give it about twenty seconds, then `taskkill //IM vfp9.exe //F`. Every
number, format and type rule in the runtime was settled this way rather than from the
documentation, and the documentation was wrong often enough to make it worth it.

## Open, reported from the IDE

Two faults seen running the Solution samples, both narrowed and neither yet fixed. Neither is a
missing feature; both are something already built getting it wrong.

- ~~A class method compiles with more in it than it has.~~ **Fixed.** It was not concatenation:
  `_table.vcx` has two `setfields`, and the second writes `DIME THIS.aMemos(THIS.iMemos)` -
  DIMENSION of an array property with parentheses, which read as a method call. The lesson is
  that the reported line number was of the method that actually failed, not the one whose name
  was in the message; read the whole library before believing the first match.
- **A form built on the foundation classes opens blank and unloads itself.** With the DIMENSION
  fault gone, `Frmsolution1` runs and then immediately destroys everything - the trace walks
  every member's Destroy and ends at Unload. Two leads. One: something in Load or Init is coming
  back false, which is how a form is cancelled, so find which. Two, and more suspicious: the
  trace says `Frmsolution1.Frmsolution1.Unload()` - a doubled path. The form's own methods are
  being addressed as `<form>.<form>.<event>` where a member's are `<form>.<member>.<event>`, so
  the form's own events are probably being looked up under a name nothing has.
- **A form's data environment cursor is not opened.** The Solution launcher stops with
  "Variable 'SOLUTIONS' is not found". Its data environment holds a Cursor whose Alias is
  `solutions` and whose CursorSource is `solution.dbf`, and the importer keeps it - the samples
  baseline only ever complained that *relations* were not imported. So this is the opening of the
  cursor when the form loads, not the import of it.

Two more, found while measuring the SQL forms a CodeMine application writes (the `sql_forms`
golden). Both belong to system 3, the resolver, and neither is fixed:

- **A LOCAL shadows a field of the same name.** Measured: with `parts` open and `LOCAL code`
  declared, the product reads `code` as the field, in a plain `? code` and in a SELECT-SQL
  WHERE alike; `m.code` is how the variable is asked for. This runtime reads the local. The
  golden avoids the case by not naming a variable after a field, and says so.
- **`_TALLY` is never set.** The product sets it after every SQL statement and after REPLACE,
  DELETE, APPEND FROM and the other record-scoped commands; here it stays at zero. Nothing
  writes it anywhere in the VM.

## Milestones

### Where the product has been

- **M1 - the IDE.** The shell and the visual designers: Form Designer with the toolbox,
  selection, snapping, page frames, grids and undo; the Properties window with VFP's own
  property names and defaults; the method editor; the Menu Designer; the Project Explorer over
  documents of our own. Nothing executed. **Done.**
- **M2 - the language.** A FoxPro compiler and a fiber-based bytecode VM in Rust, compiled to
  WebAssembly, and the runtime that puts live forms on screen: methods run, MESSAGEBOX and
  friends park the program without freezing the IDE, menus install and run, the Command Window
  and Output panel work, and Build App and Build Executable produce something that runs without
  the IDE. **Done.**
- **M3 - the data engine, the importer, and the reference.** The engine, in the main process
  where files and 64-bit offsets live: tables, indexes, relations, buffering, transactions,
  databases and SELECT-SQL. The Visual FoxPro importer beside it. Then ten waves that took the
  language reference from partly covered to **nothing unknown in any category** - 302 commands,
  403 functions, 484 properties, 147 events, 104 methods - measured by a test that reads the
  runtime's own registries, and checked against ~4,000 files of real VFP in the samples tree.
  **Done**, less the half of the engine that was left as settings, which M4 finishes.

### Where it is going

- **M4 - mostly parity.** Systems 1 to 8 and 10 below. See the cut line and the definition of
  done in the next section.
- **M5 - the long tail.** Systems 9, 11, 12, 13, 14 and 16: docking and toolbars, OLE embedding
  and the COM depth, DDE, pictures and colour, keyboard macros, help.

### M4 - mostly parity: a Visual FoxPro developer can do a day's work here

Systems **1 to 8 and 10**. What makes it the cut: after it, everything a developer touches
hourly exists - a debugger to stop in, designers for the four things VFP designs, a data engine
that uses its indexes, a resolver that runs other people's code, values that print what VFP
prints, and a build that ships. What is left after it is reached for by the week or the year.

Done when, in one sentence each:

- Every element of the reference runs, or is refused by name with what it would need. **Done.**
- A breakpoint in a form method stops there, and Locals, Watch and the call stack answer.
- A class is designed, subclassed and saved; a report is designed and printed.
- `SET PROCEDURE TO lib` and then a bare call finds the routine.
- One golden program prints the same bytes as `vfp9.exe` across the settings table.
- An indexed `SELECT` over a million records takes the time VFP takes.
- The Project Manager, Data Session and Table Designer windows are open-able.
- `BUILD EXE` produces something that runs on a machine without FoxDev.

That leaves **12 refused commands, 22 refused functions and 1 refused method** - the long tail
below, and the four that are answers rather than gaps.

### M5 - the long tail

Systems **9, 11, 12, 13, 14, 16**: docking and toolbars, OLE embedding and the COM depth,
DDE, pictures and colour, keyboard macros, help. Each is self-contained, none blocks another,
and any of them can be pulled forward if something real asks for it.

### What is left, by the system that clears it

Thirty commands, twenty-three functions and two methods are refused today. Grouped by what
would have to exist:

| System | Milestone | Commands | Functions and methods |
| --- | --- | --- | --- |
| 6 Class designer | M4 | ADD CLASS, REMOVE CLASS, CREATE CLASS, CREATE CLASSLIB | CloneObject |
| 7 Reports and the printer | M4 | CREATE REPORT, CREATE REPORT - Quick Report, CREATE LABEL, LABEL, DEFINE BOX | |
| 5 The debugger | M4 | SUSPEND, RESUME | |
| 4 The data engine | M4 | COPY INDEXES, COPY TAG | |
| 8 The IDE's windows | M4 | CREATE (Table Designer), CREATE TRIGGER | |
| 3 Names | M4 | | EXECSCRIPT |
| 10 Building | M4 | BUILD DLL, BUILD MTDLL - or a decided refusal | |
| 3, 4, 5 between them | M4 | CLOSE - its INDEXES, FORMAT, PROCEDURE, DEBUGGER and ALTERNATE forms | |
| 14 The keyboard | M5 | PLAY MACRO, SAVE MACROS, RESTORE MACROS | |
| 11 OLE and COM | M5 | APPEND GENERAL, MODIFY GENERAL | DoVerb, AMEMBERS, GETPEM, COMCLASSINFO, GETINTERFACE, EVENTHANDLER |
| 13 Graphics and colour | M5 | CREATE COLOR SET | LOADPICTURE, SAVEPICTURE, OBJTOCLIENT, RGBSCHEME, SCHEME |
| 9 Docking | M5 | DOCK | |
| 16 Help | M5 | HELP | |
| 12 DDE | M5 | | the twelve DDE functions |
| never | - | CALL, LOAD, MODIFY SCREEN, ASSIST | |

## Order

Ordered by what unblocks the most, and by what makes other people's code run at all.

| # | System | Blocks / unlocked by |
| --- | --- | --- |
| 1 | Documents of our own for classes and reports | Small; 6 and 7 author them |
| 3 | Names: procedures, class libs, paths | Other people's code does not run without it |
| 2 | Values: strings, money, code pages | Every answer the runtime gives is measured against VFP after it |
| 5 | The debugger | Machinery already exists; the first thing a developer misses |
| 4 | The data engine's second half | Needs nothing; Rushmore is where "enterprise" is won or lost |
| 6 | Class designer and Class Browser | Needs 1's class document |
| 7 | Reports, labels, printer | Needs 1's report document |
| 8 | The IDE's remaining windows | Sits beside 5 and 6 |
| 9 | Windows, docking, toolbars | Needs 8 to have something to dock |
| 10 | Building and shipping | Part built: BUILD APP/EXE/PROJECT and COMPILE run. Left: the installer and the slim player |
| 11 | COM, OLE, reflection | Independent |
| 13 | Graphics and colour | Independent, cheap |
| 14 | The keyboard | Independent |
| 16 | Help | Independent; near-free once 1 and the reference are in place |
| ~~15~~ | ~~Source control~~ | Built: git behind the SCC seam. 8 shows its status per file |
| 12 | DDE | Last, or a decided refusal |

Zero of the sixteen are element-shaped. That is the point: the remaining reference elements are
not a to-do list, they are the receipts these systems leave behind. Regenerate the map after
each one and it moves in blocks of ten and twenty, not one.

## Measurement

One correction to make first, in whichever unit lands first, because the current numbers are
wrong: `tests/reference/coverage.test.ts` classifies a `SET` option by reading the match arms of
`vm.rs::set_cmd` alone. `SET FILTER`, `SET ORDER` and `SET RELATION` are compiled as their own
statements, `SET SYSMENU`, `SET SKIP OF`, `SET MARK OF` and `SET MESSAGE` are handled in
`menu.rs`, `SET TEXTMERGE` in the parser and `SET DATABASE` in `builtins/mod.rs`. Ten options
that work are being counted as ignored, and "accepted and ignored" needs splitting into ignored
on purpose, with a reason, and not yet - the way commands already separate refused from missing.

### What the goldens measure, and what they did not

A golden program asserts what its `.expected` file says, and until now every one of those files
was written by hand. `scripts/vfp-expected.mjs` asks Visual FoxPro instead: it runs the program
under `vfp9.exe` with `SET ALTERNATE` capturing what `?` prints, and writes the file from that.
`--check` reports disagreements without writing; `--all` does every golden.

Run against all 67 goldens the first time, **57 disagreed with the product**. They fall into
four groups. Two of the four were the harness's own limits and are fixed; the sweep now reports
11 agreeing, 42 disagreeing and 14 the product cannot be asked about. With the width rule in
(below) the sweep reports 25 agreeing and 29 disagreeing, of 68.

**One rule, 29 of them: a number carries its own width.** In Visual FoxPro every numeric value
has a width and a decimal count, and `?` right-aligns to that width. A literal keeps what was
written; a variable's numeric is ten wide; arithmetic combines the operands':

    ? 8         8              ? n         (n = 4)          4
    ? 4 * 2       8            ? n * 2                        8
    ? 1.5       1.5            ? n + 1                       5
    ? 1.5 * 2     3.00         ? n / 2                    2.0000

We print the bare value formatted by `SET DECIMALS`, so `? 1.5` gives `1.50` where VFP gives
`1.5` - it is not only alignment. This means attaching a width and a decimal count to every
numeric in `value.rs` and propagating them through arithmetic, which belongs to system 2.

**Done.** `Value::Number` carries a `Width` - the characters it prints in and the places past the
point - and `?` right-aligns to it. A written number keeps what it was written with, a field the
width it was declared with, a variable ten whole digits and the places the value arrived with,
and each operator works out its answer's width. Arithmetic between two written numbers follows
the rules Visual FoxPro's compiler uses for constants, which are not the running program's, so
`? 1.5 * 2` is `  3.00` while `m = 1.5` and `? m * 2` is `           3.0`. The rules are set out
in `Width` in `value.rs` and `ref_numwidth.prg` runs them end to end against the product. Three
things fell out of measuring it: `/` by zero is not an error at all (the answer is a number too
wide to print, and `?` fills the field with asterisks - `%` by zero is still 1307), `?` ends the
open line before it evaluates so a failed expression leaves a blank one, and `^` groups left to
right with a sign binding tighter than it (`2 ^ 3 ^ 2` is 64, `-2 ^ 2` is 4).

**Four leaned on fixtures Visual FoxPro had never heard of** - the HelloWorld object tree in
`oForm`, a second program to `DO`, a report file. The harness now sets the same stage: it builds
the object tree in FoxPro source inside its wrapper, and copies
`crates/foxvm/tests/fixtures/golden` into the directory the program runs in. That directory holds
`other.prg` and the report file `PARTS.FRX`/`PARTS.FRT`, which `cargo test` keeps in step with
`dbf_fixture::sample()` - change the fixture and rerun with `REGEN_FIXTURES=1`. `doprog` agrees
with the product now, and `errors` and `with` are measured rather than assumed.

Two parts of the stage cannot be reproduced. The report file is synthesised in Rust out of nine
of an FRX's columns, and Visual FoxPro refuses it with error 1652, so `ref_reports` and
`ref_reports_self` stay uncomparable until the mock host serves a real report file. And
`APRINTERS()`/`GETPRINTER()` answer with whatever printers the machine has, never `FoxDev PDF`.

**Fifteen captured nothing, and buffering was only half the reason.** `SET ALTERNATE` does write
its buffer when the file is closed, and the error handler already closed it - six of the fifteen
were being captured all along. The other nine stopped to ask a person: `ACCEPT`, `GETCP()`,
`SUSPEND`, `READ`, `MODIFY MEMO`, `CREATE FORM`, `BUILD PROJECT`. Visual FoxPro sat there until it
was killed, and the kill is what lost the buffer.

The wrapper now carries a Timer. With `_VFP.AutoYield = .F.` Visual FoxPro fires a timer only
while it is waiting for a person - measured: a five-second busy loop produces no ticks, a modal
`ACCEPT` produces one every interval - so a tick is a reliable "this program has asked something".
Each tick closes and reopens the capture, which writes the buffer, presses Escape (the cancel the
mock host answers a dialog with), and gives up when several answers in a row leave the program
printing nothing new.

The ones that remain, and any other golden the product cannot be asked about, are listed with
their reason in `crates/foxvm/tests/programs/not-measured.txt`, which `vfp-expected.mjs --all`
writes and `golden.rs` reads: a failure on one of them says the expectation was written by hand.
Three reasons turn up. A program whose error dialog is up - `ON ERROR` with no clause puts Visual
FoxPro's own back, and no timer fires behind a modal dialog. A program the harness had to prod.
And a program whose output names the directory it ran in, because Visual FoxPro answers with full
paths where the mock host answers with bare names.

**Nine are real behaviour we get wrong**, each worth its own fix:

- `_INCSEEK` is 0.50, not 0.30.
- ~~`? 1/0` prints `****************`; it is not error 1307.~~ Done with the width work.
- `THROW "boom"` leaves `MESSAGE()` as `User Thrown Error .`, not `boom`.
- `LIST` prints a header row (`Record#  CODE NAME  PRICE`) before the records.
- `FIELD()` and the listing commands answer with the field name upper-cased.
- ~~A numeric field prints with the decimals the column was defined with (`3200.00`, not
  `3200`).~~ Done with the width work.
- An error message ends with a full stop and pads the code: `caught 1734 Property  is not found.`
- `ref_object` and `ref_screen` differ in value, not only in width.
- A macro in a clause position prints unexpanded in one place (`macro_positions`).

Measuring the ones that used to capture nothing turned up more, not yet split out of the count:

- `PROGRAM()` upper-cases the routine name whatever the source says, so an unhandled error in
  `PROCEDURE Broken` reads `in BROKEN`. `SYS(16)` does the same.
- `?` writes the newline that ends the open line _before_ it evaluates what follows, so an
  expression that fails leaves a blank line behind (`retry`, `release`, `scoping`).
- `m.` is the memory-variable prefix, so `CATCH TO m` followed by `m.Message` reads the memvar
  `Message` and not the exception's property. Three goldens name their exception `m`.
- A built-in wins a name a program also uses: `? Second()` beside `PROCEDURE Second` answers with
  the seconds since midnight.
- A `Form` made with `CREATEOBJECT` has no `Parent`; asking for it is error 1924, where the mock
  host answers `.NULL.`.

The lesson is the one the reference already taught: what is checked in is only ground truth if
the product produced it. The existing goldens need working through group by group.

### Making a golden

1. Write the `.prg` in `crates/foxvm/tests/programs`. It may use the stage the runner sets: the
   HelloWorld tree in `oForm`, `DO other`, the `PARTS` report.
2. Run `node scripts/vfp-expected.mjs crates/foxvm/tests/programs/<name>.prg`. It runs the program
   under `vfp9.exe` and writes the `.expected` from what the product printed. If it answers
   `NOT MEASURED` instead, the program needs a person: its name goes into `not-measured.txt` with
   the reason, and the `.expected` has to be written by hand knowing that.
3. Commit both files, and `not-measured.txt` if it changed.

`npm run goldens:check` sweeps every golden and reports what disagrees with the product without
touching a `.expected`. It needs Visual FoxPro 9 installed, so it is not part of `npm run check`.

A `COVERS:` name may be written `method:Help` where the reference has that name twice in two
kinds. A bare name covers every element called that, which is what nearly all of them mean; the
application's `Help` and `Quit` methods would otherwise quietly cover the `HELP` and `QUIT`
commands, which open a window and end the session and are not exercised anywhere.

### The three classes with no golden, and why

`Application`, `ReportListener`, `OLE Container` and `OLE Bound` have one now
(`ref_class_application`, `ref_class_reportlistener`, `ref_class_ole`), and their members are
measured into `tests/reference/vfp-base-classes.tsv` along with `Project`, `File` and `FormSet`.
Three of the reference's objects still have none:

- **`Project` and `File`.** Both are measured - a project is made with `CREATE PROJECT x NOWAIT
  NOSHOW` and asked for as `_VFP.ActiveProject` - but a golden runs against the VM's own mock
  host, which has no project. Covering them means the VM making a project, which is a piece of
  system 8 and not a golden. Twenty-eight properties and ten methods are waiting behind it.
- **`Server`.** A server is an OLEPUBLIC class in a project that has been **built** into a COM
  server. The build registers it under `HKEY_CLASSES_ROOT`, which an unelevated process cannot
  write; the DLL and the type library are produced, the registration silently fails, and the
  project's `Servers` collection stays empty. It needs an elevated run to measure at all.
- **`DataObject`.** It exists only for the length of an OLE drag: `CREATEOBJECT("DataObject")` is
  error 1733, `OLEDrag()` called from a program does not start one, and `OLEStartDrag` never
  fires without a person holding the mouse.

Three properties the reference names are on none of the product's classes, measured: `Align` and
`Object` belong to an OLE container **holding an ActiveX control**, which is a different shape
from the one holding a document - it takes the control's own members and has none of AutoActivate,
AutoVerbMenu, DocumentFile or HostName - and `ReadObject` the product has nowhere at all.

### Open: a relation into a file-backed table lands at EOF

A form's data environment now reads its `relation` rows and sets them up (`SET RELATION TO
<expr> INTO <child> ADDITIVE`, with `SET SKIP` for one-to-many), which took the samples corpus
from 44 issues to 26. The link works over cursors held in memory - `crates/foxvm` runs it
correctly from the CLI - but over a table paged in from a file the child lands on
`RECCOUNT() + 1` instead of the record the expression names. Reading the child's record needs a
host request, and the instruction that applies the relation appears not to be able to suspend
for one. The end-to-end test for it is written and waiting on the fix.

Two things found on the way, both worth fixing on their own:

- `Scheduler.handleError`'s **synchronous** branch resumes the fiber from inside `drive`'s own
  loop. Nothing took that branch before, because `onError` always returned a promise; the first
  caller that returned a bare `'ignore'` aborted the wasm with SIGABRT. The data environment
  now guards itself with FoxPro's own `TRY ... CATCH` instead, so no error reaches the hook, but
  the branch is still there and still wrong.
- A form's data environment must never raise the error dialog. It runs while the form is being
  built, and the dialog waits for an answer nobody is there to give.

### Open, found by the goldens once the product wrote them

Three differences the golden sweep turned up that are recorded rather than fixed, each with a
golden whose checked-in expectation is ours and whose measured one is in this note:

- **An error raised inside a `CATCH` block.** The product reports 2059 at the `ENDTRY` and stops;
  we report the error itself, at its own line, and carry on. `trycatch.prg` records ours.

And one the merge itself found and fixed: **`m.` is the memory-variable prefix and wins even over
a variable called `m`**, so `CATCH TO m` then `m.Message` reads a variable called `Message`. Three
goldens and a VM test were written on our wrong reading of it.
