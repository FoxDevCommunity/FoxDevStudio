// Asks Visual FoxPro 9 what each of its base classes is made of, and writes the answer to
// `tests/reference/vfp-base-classes.tsv`.
//
// The property sheet in the reference is prose; the object in the product is a fact. This walks
// every base class a program can say `CREATEOBJECT` about, and for each one writes down every
// member it answers to - properties with the value they start out holding, events and methods by
// name - so that our registry can be checked against the product rather than against a memory of
// it.
//
//   node scripts/vfp-base-classes.mjs             # rewrites the table
//   node scripts/vfp-base-classes.mjs --check     # writes nothing, reports what changed
//   node scripts/vfp-base-classes.mjs --dump p.prg  # writes out the probe rather than running it
//
// How it works. `AMEMBERS(a, o, 1)` is the product's own member list, so the names are its and
// not ours. It leaves out a few properties an object still answers to - `Application` is the one
// that matters - so a second pass asks `PEMSTATUS` about those by name. Two objects of the same
// class are made and compared: a property that already differs between two fresh objects is not
// a default at all (a window handle is the case that matters), and is written down as `varies`.
// Reading some properties raises rather than answering - `Parent` on an object with no parent,
// `Controls` on a container with nothing in it - and those are written down as `error`.
//
// Not every class answers to `CREATEOBJECT`. The application is `_VFP` and there is one of it; a
// project has to be made on disk first and asked for as `_VFP.ActiveProject`; a file is something
// in a project; an OLE container needs a form to live on and a control to hold. Each of those is
// in `REACHED` with the way the product does hand one over, and is then measured exactly as the
// rest are. Such an object comes out of a type library rather than out of FoxPro's own class
// engine, and answers `AMEMBERS(a, o, 1)` with almost nothing, so the member list is whichever of
// `AMEMBERS(a, o, 1)` and `AMEMBERS(a, o, 3)` is longer.
//
// The whole probe is run twice, in a fresh directory each time, and a value the two runs disagree
// about is `varies` as well. That is what catches the things there is only one of - the
// application's process id, its window handle, the directory it was started in - which comparing
// two references to one object never could.
//
// Two classes are still missing and cannot be measured here. A `DataObject` exists only for the
// length of an OLE drag, which needs a person with a mouse. A `Server` is a class marked
// OLEPUBLIC in a project that has been built into a COM server, and the build registers it under
// HKEY_CLASSES_ROOT, which an unelevated process cannot write; without the registration the
// project's `Servers` collection stays empty.
//
// A second pass then asks whether each property can be written to, by writing back the value it
// already holds. It runs on the second object, which nothing reads again, because even a write
// that changes nothing can change what an object says about itself afterwards. Writing a
// different value would be the better question, but on a window class it shows the window and
// the probe then waits for a person for ever; this way under-reports rather than hanging, so a
// property the product refuses only when the value really changes is not caught. The error it
// raised, or nothing, is the `readonly` column.
//
// Visual FoxPro must be installed. Nothing here runs in CI; it is how the checked-in table is
// made, and `tests/shared/baseClasses.test.ts` is what then holds the registry to it.

import { execFile, execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { clearTimeout, setTimeout } from 'node:timers';

const VFP = 'C:/Program Files (x86)/Microsoft Visual FoxPro 9/vfp9.exe';
const ROOT = resolve(import.meta.dirname, '..');
const TABLE = join(ROOT, 'tests/reference/vfp-base-classes.tsv');
const TIMEOUT_MS = 240_000;

/**
 * Every base class a program can create with no arguments.
 *
 * The classes a bare `CREATEOBJECT` cannot make are in `REACHED` below, each with the way the
 * product does hand one over.
 */
const CLASSES = [
  'Form', 'Label', 'TextBox', 'EditBox', 'CommandButton', 'CommandGroup', 'CheckBox',
  'OptionGroup', 'OptionButton', 'ComboBox', 'ListBox', 'Spinner', 'Container', 'PageFrame',
  'Page', 'Grid', 'Column', 'Header', 'Image', 'Line', 'Shape', 'Timer', 'Custom', 'Session',
  'Collection', 'Exception', 'Empty', 'Relation', 'Cursor', 'DataEnvironment', 'Toolbar',
  'Separator', 'Hyperlink', 'CursorAdapter', 'Control', 'ProjectHook', 'XMLAdapter', 'XMLTable',
  'XMLField', 'ReportListener', 'FormSet',
];

/**
 * The classes `CREATEOBJECT` refuses, and what the product does hand one of them over for.
 *
 * `setup` is run once, before any of them is asked about; `a` and `b` are the two objects the
 * comparison needs. Two of a kind still, wherever the product will make two: two projects, two
 * files of one, two OLE containers around different controls - so that a value which depends on
 * what the object was reached through is seen not to be a default. The application is the
 * exception, because there is only ever one of it; the second run (below) is what catches its
 * process id and its working directory instead.
 */
const REACHED = [
  // there is one copy of the product and `_VFP` is it
  { class: 'Application', setup: [], a: '_VFP', b: '_VFP' },
  // A project has to exist on disk before there is an object for it. Everything from here on
  // reaches the project manager, and the project manager puts up windows: `NOWAIT NOSHOW` keeps
  // it out of sight, every file it is given is named in full so it never has to ask which one
  // was meant, and the keeper starts here so that a window nothing expected is answered rather
  // than left standing. It cannot start any earlier because it turns `AutoYield` off, and
  // `AutoYield` is one of the application's own properties this is measuring.
  {
    class: 'Project',
    setup: [
      '_VFP.AutoYield = .F.',
      'goKeeper = CREATEOBJECT("fdvKeeper")',
      'CREATE PROJECT one NOWAIT NOSHOW',
      'oPrjA = _VFP.ActiveProject',
      'CREATE PROJECT two NOWAIT NOSHOW',
      'oPrjB = _VFP.ActiveProject',
    ],
    a: 'oPrjA',
    b: 'oPrjB',
  },
  // and a file is something in a project
  {
    class: 'File',
    setup: [
      'STRTOFILE("* one" + CHR(13) + CHR(10), "one.prg")',
      'STRTOFILE("* two" + CHR(13) + CHR(10), "two.prg")',
      'oPrjA.Files.Add(FULLPATH("one.prg"))',
      'oPrjA.Files.Add(FULLPATH("two.prg"))',
    ],
    a: 'oPrjA.Files.Item(1)',
    b: 'oPrjA.Files.Item(2)',
  },
  // An OLE container needs a form to live on and something to hold, and what it holds decides
  // what it answers to. Two of them holding different documents, so that what belongs to the
  // container is told apart from what belongs to the document in it.
  //
  // A container holding an ActiveX control instead is a different shape: it takes the control's
  // own properties and events, and has none of AutoActivate, AutoVerbMenu, DocumentFile or
  // HostName - measured on a WebBrowser and a Media Player. The documented OLE Container is the
  // one that holds a document, so that is the one written down; the members an ActiveX brings
  // are its, and the object model finds them on the control.
  {
    class: 'OleControl',
    viaAdd: true,
    setup: [
      'oOleFrm = CREATEOBJECT("Form")',
      'oOleFrm.AddObject("olea", "OleControl", "Paint.Picture")',
      'oOleFrm.AddObject("oleb", "OleControl", "Package")',
    ],
    a: 'oOleFrm.olea',
    b: 'oOleFrm.oleb',
  },
  // a bound one takes its control from the field it is bound to, so it needs no name
  {
    class: 'OleBoundControl',
    viaAdd: true,
    setup: ['oOleFrm.AddObject("bounda", "OleBoundControl")', 'oOleFrm.AddObject("boundb", "OleBoundControl")'],
    a: 'oOleFrm.bounda',
    b: 'oOleFrm.boundb',
  },
];

/**
 * Properties `AMEMBERS` does not list even though the object answers to them.
 *
 * There is no way to ask the product for this list, so it is the names our own registry knows
 * about that the member list never mentions; each is asked about by name instead. `PEMSTATUS`
 * does not always know either - an OLE container answers `Object` and denies having it - so the
 * question is whether reading it answers.
 */
const UNLISTED = [
  'Application', 'hWnd', 'ControlCount', 'Controls', 'Objects', 'Count', 'Object', 'Align',
];

/**
 * What `AddObject` writes on the control it adds, which is not the class answering differently.
 *
 * It names the control, and a class whose caption follows its name takes that too; it gives it
 * the next tab order; and it adds it hidden, because the form is not showing. Everything else a
 * contained object answers differently is the class's own answer to being in a container.
 */
const WRITTEN_BY_ADD = ['NAME', 'CAPTION', 'TABINDEX', 'VISIBLE'];

/** The probe: what runs inside Visual FoxPro. */
function probeSource() {
  const lines = [
    'ON ERROR ?? ""',
    'SET SAFETY OFF',
    'SET TALK OFF',
    'SET ESCAPE OFF',
    'PUBLIC gcOut, oPrjA, oPrjB, oOleFrm, goKeeper',
    'gcOut = ""',
    'LOCAL i, oA, oB',
    `LOCAL ARRAY aCls[${CLASSES.length}]`,
    ...CLASSES.map((c, i) => `aCls[${i + 1}] = "${c}"`),
    'FOR i = 1 TO ALEN(aCls)',
    '  oA = .NULL.',
    '  oB = .NULL.',
    '  TRY',
    '    oA = CREATEOBJECT(aCls[i])',
    '    oB = CREATEOBJECT(aCls[i])',
    '  CATCH',
    '  ENDTRY',
    '  IF ISNULL(oA) OR ISNULL(oB)',
    '    gcOut = gcOut + "!" + CHR(9) + aCls[i] + CHR(13) + CHR(10)',
    '  ELSE',
    `    DO OneClass WITH aCls[i], "${UNLISTED.join(',')}", oA, oB, ""`,
    '  ENDIF',
    'ENDFOR',
    // the classes CREATEOBJECT refuses, each reached the way the product does hand one over
    ...REACHED.flatMap((r) => [
      ...r.setup.flatMap((line) => ['TRY', `  ${line}`, 'CATCH', 'ENDTRY']),
      'oA = .NULL.',
      'oB = .NULL.',
      'TRY',
      `  oA = ${r.a}`,
      `  oB = ${r.b}`,
      'CATCH',
      'ENDTRY',
      'IF ISNULL(oA) OR ISNULL(oB)',
      `  gcOut = gcOut + "!" + CHR(9) + "${r.class}" + CHR(13) + CHR(10)`,
      'ELSE',
      `  DO OneClass WITH "${r.class}", "${UNLISTED.join(',')}", oA, oB, "${r.viaAdd ? WRITTEN_BY_ADD.join(',') : ''}"`,
      'ENDIF',
    ]),
    // A control in a form is not the same object as one on its own: the colours it takes from
    // the system's 3D palette are resolved against the container, so a Label answers white until
    // it is placed and 15790320 once it is. Every control on every form is contained, so that
    // second answer is the one a form is drawn from.
    'FOR i = 1 TO ALEN(aCls)',
    '  DO OneContained WITH aCls[i]',
    'ENDFOR',
    'STRTOFILE(gcOut, "classes.txt")',
    'STRTOFILE("done" + CHR(13) + CHR(10), "how.txt")',
    'QUIT',
    '',
    '* The keeper. With AutoYield off Visual FoxPro fires a timer only while it is waiting for a',
    '* person, so a tick means something has put a window up. Escape is the answer a dialog gets;',
    '* what has been measured so far is written out on every tick, so a run that has to be ended',
    '* still shows how far it got, and a probe that stops moving ends itself rather than standing',
    '* there in front of whoever is at the machine.',
    'DEFINE CLASS fdvKeeper AS Timer',
    '  Interval = 400',
    '  nSeen = 0',
    '  nQuiet = 0',
    '  PROCEDURE Timer',
    '    STRTOFILE(gcOut, "classes.txt")',
    '    KEYBOARD "{ESC}"',
    '    IF LEN(gcOut) > THIS.nSeen',
    '      THIS.nSeen = LEN(gcOut)',
    '      THIS.nQuiet = 0',
    '    ELSE',
    '      THIS.nQuiet = THIS.nQuiet + 1',
    '    ENDIF',
    '    IF THIS.nQuiet > 25',
    '      STRTOFILE("stalled" + CHR(13) + CHR(10), "how.txt")',
    '      QUIT',
    '    ENDIF',
    '  ENDPROC',
    'ENDDEFINE',
    '',
    '* One class: two objects of it, so that a value which is not the same in both is seen not',
    '* to be a default at all.',
    'PROCEDURE OneClass',
    'LPARAMETERS cCls, cUnlisted, oA, oB, cUnmeasurable',
    'LOCAL j, cName, cKind, cSeen, cMine, cTheirs, cProps',
    'cSeen = ","',
    'cProps = ""',
    'cMine = MemberList(oA)',
    'cTheirs = "," + UPPER(MemberList(oB))',
    '* one row per class that could be made, so that a class with no members at all - Empty is',
    '* one - is known to have none rather than not to have been asked about',
    'gcOut = gcOut + cCls + CHR(9) + "class" + CHR(9) + CHR(9) + CHR(9) + CHR(13) + CHR(10)',
    'FOR j = 1 TO GETWORDCOUNT(cMine, ",")',
    '  cName = GETWORDNUM(GETWORDNUM(cMine, j, ","), 1, ":")',
    '  cKind = GETWORDNUM(GETWORDNUM(cMine, j, ","), 2, ":")',
    '  * a member the other one has not got belongs to what this object was reached through, not',
    '  * to its class: the control an OLE container happens to hold is the case that matters',
    '  IF NOT ("," + UPPER(cName) + ":") $ cTheirs',
    '    LOOP',
    '  ENDIF',
    '  cSeen = cSeen + UPPER(cName) + ","',
    '  IF cKind == "property"',
    '    cProps = cProps + cName + ","',
    '    * A class that can only be reached by adding it to a form is measured as AddObject left',
    '    * it: named, tab-ordered and hidden, because the form is not showing. None of that is the',
    '    * class answering, and there is no unparented one to ask, so it is written down as having',
    '    * no default rather than as having that one.',
    '    IF ("," + UPPER(cName) + ",") $ ("," + UPPER(cUnmeasurable) + ",") AND NOT EMPTY(cUnmeasurable)',
    '      gcOut = gcOut + cCls + CHR(9) + "property" + CHR(9) + cName + CHR(9) + "varies" + CHR(9) + CHR(13) + CHR(10)',
    '    ELSE',
    '      DO OneProp WITH cCls, cName, oA, oB',
    '    ENDIF',
    '  ELSE',
    '    gcOut = gcOut + cCls + CHR(9) + cKind + CHR(9) + cName + CHR(9) + CHR(9) + CHR(13) + CHR(10)',
    '  ENDIF',
    'ENDFOR',
    '* the few the member list leaves out, asked for by name',
    'FOR j = 1 TO GETWORDCOUNT(cUnlisted, ",")',
    '  cName = GETWORDNUM(cUnlisted, j, ",")',
    '  IF Answers(oA, cName) AND NOT ("," + UPPER(cName) + ",") $ cSeen',
    '    cSeen = cSeen + UPPER(cName) + ","',
    '    cProps = cProps + cName + ","',
    '    DO OneProp WITH cCls, cName, oA, oB',
    '  ENDIF',
    'ENDFOR',
    '* Second pass, over the same properties: whether each can be written to. It goes over oB,',
    '* which nothing reads again, because a write can change what an object says about itself',
    '* afterwards.',
    'FOR j = 1 TO GETWORDCOUNT(cProps, ",")',
    '  DO OneWrite WITH cCls, GETWORDNUM(cProps, j, ","), oB',
    'ENDFOR',
    'ENDPROC',
    '',
    '* Every member of one object, as "Name:kind,Name:kind," - kind being property, event or',
    '* method. Neither a member name nor a kind can hold a comma or a colon, so the list reads',
    '* back word by word.',
    '*',
    '* AMEMBERS(a, o, 1) is the product\'s own member list, but an object it hands over out of a',
    '* type library - the application itself, a project, a file of one - answers that with almost',
    '* nothing and answers AMEMBERS(a, o, 3) with the library. So both are asked and the longer',
    '* answer wins, which leaves a native class on its own list: an OLE container answers 3 with',
    '* the control it holds, and that is the shorter of the two.',
    'FUNCTION MemberList',
    'LPARAMETERS oObj',
    'LOCAL j, n1, n3, cOut, cName',
    'LOCAL ARRAY aOwn[1], aLib[1]',
    'n1 = 0',
    'n3 = 0',
    'TRY',
    '  n1 = AMEMBERS(aOwn, oObj, 1)',
    'CATCH',
    'ENDTRY',
    'TRY',
    '  n3 = AMEMBERS(aLib, oObj, 3)',
    'CATCH',
    'ENDTRY',
    'cOut = ","',
    'IF n3 > n1',
    '  * a type library names a property once for reading and again for writing',
    '  FOR j = 1 TO n3',
    '    cName = aLib[j, 1]',
    '    IF NOT ("," + UPPER(cName) + ":") $ UPPER(cOut)',
    '      cOut = cOut + cName + ":" + IIF(LOWER(aLib[j, 2]) == "method", "method", "property") + ","',
    '    ENDIF',
    '  ENDFOR',
    'ELSE',
    '  FOR j = 1 TO n1',
    '    cOut = cOut + aOwn[j, 1] + ":" + IIF(LEFT(LOWER(aOwn[j, 2]), 1) == "p", "property", LOWER(aOwn[j, 2])) + ","',
    '  ENDFOR',
    'ENDIF',
    'RETURN SUBSTR(cOut, 2)',
    'ENDFUNC',
    '',
    '* Whether an object has a member of that name.',
    '*',
    '* PEMSTATUS is the question, and reading the name is not a second one: an OLE container is a',
    '* dispatch of its own and answers .NULL. to every name there is, so a read that works proves',
    '* nothing about it.',
    'FUNCTION Answers',
    'LPARAMETERS oObj, cName',
    'RETURN PEMSTATUS(oObj, cName, 5)',
    'ENDFUNC',
    '',
    '* Whether the product lets one property be written to. The value written is the one the',
    '* property already holds: a different one would show a window, dock a toolbar or minimise',
    '* something, and a probe that puts a window up waits for a person for ever.',
    'PROCEDURE OneWrite',
    'LPARAMETERS cCls, cName, oB',
    'LOCAL cType, uOld, cLine, cErr',
    'cErr = ""',
    'TRY',
    '  uOld = EVALUATE("oB." + cName)',
    '  cType = VARTYPE(uOld)',
    '  IF NOT INLIST(cType, "O", "X")',
    '    cLine = "oB." + cName + " = uOld"',
    '    &cLine',
    '  ENDIF',
    'CATCH TO oW',
    '  cErr = LTRIM(STR(oW.ErrorNo))',
    'ENDTRY',
    'IF NOT EMPTY(cErr)',
    '  gcOut = gcOut + cCls + CHR(9) + "write" + CHR(9) + cName + CHR(9) + cErr + CHR(13) + CHR(10)',
    'ENDIF',
    'ENDPROC',
    '',
    '',
    '* What one property holds once the object is in a container, where that is not what it holds',
    '* on its own. Two forms, so a value that differs between two contained objects is left alone',
    '* exactly as the first pass leaves one that differs between two bare objects.',
    'PROCEDURE OneContained',
    'LPARAMETERS cCls',
    'LOCAL oBare, oFrmA, oFrmB, oInA, oInB, j, cName, cType, cBare, cInA, cInB',
    'LOCAL ARRAY aC[1]',
    'oBare = .NULL.',
    'oFrmA = .NULL.',
    'TRY',
    '  oBare = CREATEOBJECT(cCls)',
    '  oFrmA = CREATEOBJECT("Form")',
    '  oFrmB = CREATEOBJECT("Form")',
    '  oFrmA.AddObject("ocheld", cCls)',
    '  oFrmB.AddObject("ocheld", cCls)',
    'CATCH',
    'ENDTRY',
    'IF ISNULL(oBare) OR ISNULL(oFrmA) OR NOT PEMSTATUS(oFrmA, "ocheld", 5)',
    '  RETURN',
    'ENDIF',
    'oInA = oFrmA.ocheld',
    'oInB = oFrmB.ocheld',
    'FOR j = 1 TO AMEMBERS(aC, oBare, 1)',
    '  IF NOT LEFT(LOWER(aC[j, 2]), 1) == "p"',
    '    LOOP',
    '  ENDIF',
    '  cName = aC[j, 1]',
    `  IF ("," + UPPER(cName) + ",") $ ",${WRITTEN_BY_ADD.join(',')},"`,
    '    LOOP',
    '  ENDIF',
    '  cBare = AsText(oBare, cName)',
    '  cInA = AsText(oInA, cName)',
    '  cInB = AsText(oInB, cName)',
    '  * unchanged, unsettled, or not an answer at all',
    '  IF cInA == cBare OR NOT (cInA == cInB) OR LEFT(cInA, 1) == "?"',
    '    LOOP',
    '  ENDIF',
    '  cType = "?"',
    '  TRY',
    '    cType = VARTYPE(EVALUATE("oInA." + cName))',
    '  CATCH',
    '  ENDTRY',
    '  gcOut = gcOut + cCls + CHR(9) + "contained" + CHR(9) + cName + CHR(9) + cType + CHR(9) + SUBSTR(cInA, 2) + CHR(13) + CHR(10)',
    'ENDFOR',
    'ENDPROC',
    '',
    '* One property as text, with a marker in front so that an error and an empty string are not',
    '* the same answer.',
    'FUNCTION AsText',
    'LPARAMETERS oObj, cName',
    'LOCAL u',
    'TRY',
    '  u = EVALUATE("oObj." + cName)',
    'CATCH',
    '  RETURN "?error"',
    'ENDTRY',
    'DO CASE',
    'CASE INLIST(VARTYPE(u), "C", "M")',
    '  RETURN "=" + u',
    'CASE VARTYPE(u) == "O"',
    '  RETURN "?object"',
    'OTHERWISE',
    '  RETURN "=" + TRANSFORM(u)',
    'ENDCASE',
    'ENDFUNC',
    '* What one property holds on a brand new object, as text, with its type beside it.',
    'PROCEDURE OneProp',
    'LPARAMETERS cCls, cName, oA, oB',
    'LOCAL cType, cVal, uA, uB',
    'cType = "?"',
    'cVal = ""',
    'TRY',
    '  uA = EVALUATE("oA." + cName)',
    '  uB = EVALUATE("oB." + cName)',
    '  cType = VARTYPE(uA)',
    '  DO CASE',
    '  CASE INLIST(cType, "C", "M")',
    '    cVal = uA',
    '  CASE cType == "O"',
    '    cVal = "object"',
    '  OTHERWISE',
    '    cVal = TRANSFORM(uA)',
    '  ENDCASE',
    '  IF NOT (cType == VARTYPE(uB)) OR NOT (cVal == IIF(INLIST(VARTYPE(uB), "C", "M"), uB, IIF(VARTYPE(uB) == "O", "object", TRANSFORM(uB))))',
    '    cType = "varies"',
    '    cVal = ""',
    '  ENDIF',
    'CATCH',
    '  cType = "error"',
    '  cVal = ""',
    'ENDTRY',
    'gcOut = gcOut + cCls + CHR(9) + "property" + CHR(9) + cName + CHR(9) + cType + CHR(9) + cVal + CHR(13) + CHR(10)',
    'ENDPROC',
    '',
  ];
  return lines.join('\r\n');
}

/** Runs the probe in a directory of its own and answers with what it wrote. */
async function askVfp() {
  const dir = mkdtempSync(join(tmpdir(), 'fdv-classes-'));
  try {
    writeFileSync(join(dir, 'probe.prg'), probeSource(), 'latin1');
    await new Promise((done) => {
      const child = execFile(VFP, [join(dir, 'probe.prg')], { cwd: dir }, () => done());
      const timer = setTimeout(() => {
        try {
          child.kill();
        } catch {
          // it may already be gone; the taskkill below is what actually ends a modal one
        }
        done();
      }, TIMEOUT_MS);
      child.on('exit', () => {
        clearTimeout(timer);
        done();
      });
    });
    // whether it finished or not: a window this probe did not expect belongs to nobody, and the
    // person at the machine should not be the one who has to close it
    try {
      execFileSync('taskkill', ['/IM', 'vfp9.exe', '/F'], { stdio: 'ignore' });
    } catch {
      // nothing left to kill, which is the good case
    }
    // a probe that does not compile leaves its complaint in a .ERR file and writes nothing
    const err = join(dir, 'probe.ERR');
    if (existsSync(err)) return { error: readFileSync(err, 'latin1').trim() };
    const out = join(dir, 'classes.txt');
    if (!existsSync(out)) return { error: 'Visual FoxPro wrote nothing; it may have stopped to ask a person' };
    const how = join(dir, 'how.txt');
    if (!existsSync(how)) return { error: 'Visual FoxPro never reached the end of the probe and was killed' };
    const ending = readFileSync(how, 'latin1').trim();
    if (ending !== 'done') return { error: `the probe gave up: ${ending}` };
    return { text: readFileSync(out, 'latin1') };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** One run's answer, read into a member-by-member map and the list of classes it refused. */
function readRun(text) {
  const members = new Map();
  const refused = [];
  // the write pass comes back as its own rows; each is the error one property's write raised,
  // and belongs in the column beside that property
  const onWrite = new Map();
  for (const line of text.split(/\r?\n/)) {
    if (line.trim() === '') continue;
    if (line.startsWith('!')) {
      refused.push(line.split('\t')[1]);
      continue;
    }
    const [cls, kind, name, type, value = ''] = line.split('\t');
    if (kind === 'write') {
      onWrite.set(`${cls}\t${name.toUpperCase()}`, type);
      continue;
    }
    members.set(`${cls}\t${kind}\t${name.toUpperCase()}`, [cls, kind, name, type, value, '']);
  }
  for (const row of members.values()) {
    if (row[1] === 'property') row[5] = onWrite.get(`${row[0]}\t${row[2].toUpperCase()}`) ?? '';
  }
  return { members, refused };
}

/**
 * The table, sorted so that a re-measurement diffs against the last one line by line.
 *
 * Two runs go in, and a member is written down only as far as both of them agree: a value the
 * second run answered differently is no more a default than one two objects of a class disagreed
 * about, and is written down as `varies` in the same way. That is what catches the things there
 * is only one of - the running application's process id, its window handle, the directory it
 * happens to have been started in - which two references to the same object never could.
 */
function table(runs) {
  const [first, ...rest] = runs.map(readRun);
  const refused = first.refused;
  const rows = [];
  const dropped = [];
  for (const [key, row] of first.members) {
    const others = rest.map((r) => r.members.get(key));
    if (others.some((o) => o === undefined)) {
      dropped.push(key.replace(/\t/g, ' '));
      continue;
    }
    // type, default and the write's answer all have to hold up; a value that did not is not one
    if (others.some((o) => o[3] !== row[3] || o[4] !== row[4])) {
      row[3] = 'varies';
      row[4] = '';
    }
    if (others.some((o) => o[5] !== row[5])) row[5] = '';
    rows.push(row);
  }
  if (dropped.length) {
    console.error(`only one run of the product knew about: ${dropped.join(', ')}`);
  }
  const order = new Map([...CLASSES, ...REACHED.map((r) => r.class)].map((c, i) => [c, i]));
  const kinds = { class: 0, property: 1, event: 2, method: 3 };
  rows.sort(
    (a, b) =>
      (order.get(a[0]) ?? 99) - (order.get(b[0]) ?? 99) ||
      (kinds[a[1]] ?? 9) - (kinds[b[1]] ?? 9) ||
      a[2].localeCompare(b[2]),
  );
  const head = [
    '# Every Visual FoxPro 9 base class, as the product itself answers about it.',
    '# Written by `node scripts/vfp-base-classes.mjs`; read by tests/shared/baseClasses.test.ts.',
    '#',
    '# type is the VARTYPE letter of the value a brand new object holds, "varies" when two fresh',
    '# objects - or two runs of the product - already disagree (a window handle, a process id),',
    '# and "error" when reading it raises instead.',
    '# readonly is the error the product raised when the property was written the value it already',
    '# held, and empty when it took the write. 1743 is FoxPro refusing one of its own; 1429 with',
    '# dispatch code 533 is an object out of a type library - the application, a project - saying',
    '# the same thing through OLE.',
    '#',
    '# A few of the colours - the selected-item ones - are the Windows theme the measuring machine',
    '# was wearing, so a re-measurement somewhere else will differ in those and nothing else.',
    'class\tkind\tname\ttype\tdefault\treadonly',
  ];
  if (refused.length) head.splice(4, 0, `# no object could be made of: ${refused.join(', ')}`);
  return head.concat(rows.map((r) => r.join('\t')), '').join('\n');
}

async function main() {
  if (process.argv.includes('--dump')) {
    writeFileSync(process.argv[process.argv.indexOf('--dump') + 1], probeSource(), 'latin1');
    return;
  }
  if (!existsSync(VFP)) {
    console.error(`Visual FoxPro 9 is not installed at ${VFP}; nothing to ask.`);
    process.exit(2);
  }
  // twice, in two directories of its own, so that a value which is only true of one run of the
  // product is seen not to be a default
  const answers = [await askVfp(), await askVfp()];
  for (const answer of answers) {
    if (answer.error) {
      console.error(answer.error);
      process.exit(1);
    }
  }
  const made = table(answers.map((a) => a.text));
  const had = existsSync(TABLE) ? readFileSync(TABLE, 'latin1') : null;
  if (had === made) {
    console.log(`${TABLE}: agrees with the product`);
    return;
  }
  if (process.argv.includes('--check')) {
    console.log(`${TABLE}: DIFFERS from the product; rerun without --check to rewrite it`);
    process.exit(1);
  }
  writeFileSync(TABLE, made, 'latin1');
  console.log(`${TABLE}: written from Visual FoxPro (${made.split('\n').length - 7} member(s))`);
}

await main();
