mod common;

use common::{dump, messages, ok};
use foxvm::ast::{ProcKind, StmtKind};
use foxvm::diagnostics::Severity;
use foxvm::parser::{parse_method, parse_program};

#[test]
fn hello_world_main_prg() {
    assert_eq!(
        ok("* HelloWorld main program\nDO FORM HelloWorld\nREAD EVENTS\n"),
        "(doform \"HelloWorld\")\n(read-events)"
    );
}

#[test]
fn hello_world_form_methods() {
    let init = "* runs when the form is created\nTHISFORM.txtName.SetFocus()";
    let out = parse_method(init);
    assert!(out.diagnostics.is_empty());
    assert_eq!(dump(&out.program), "(expr THISFORM.TXTNAME.SETFOCUS())");
    assert_eq!(out.program.body.stmts[0].line, 2);

    let click = "LOCAL cMsg\ncMsg = \"Hello, \" + ALLTRIM(THISFORM.txtName.Value)\nIF THISFORM.chkLoud.Value\n  cMsg = UPPER(cMsg) + \"!\"\nENDIF\nTHISFORM.pgfMain.Page1.lblGreeting.Caption = cMsg";
    let out = parse_method(click);
    assert!(out.diagnostics.is_empty());
    assert_eq!(
        dump(&out.program),
        "(local CMSG)\n\
         (= CMSG (+ \"Hello, \" ALLTRIM(THISFORM.TXTNAME.VALUE)))\n\
         (if THISFORM.CHKLOUD.VALUE (block (= CMSG (+ UPPER(CMSG) \"!\"))))\n\
         (= THISFORM.PGFMAIN.PAGE1.LBLGREETING.CAPTION CMSG)"
    );
    let lines: Vec<u32> = out.program.body.stmts.iter().map(|s| s.line).collect();
    assert_eq!(lines, vec![1, 2, 3, 6]);

    let close = "THISFORM.Release()";
    let out = parse_method(close);
    assert!(out.diagnostics.is_empty());
    assert_eq!(dump(&out.program), "(expr THISFORM.RELEASE())");
}

#[test]
fn menu_commands() {
    assert_eq!(ok("DO FORM HelloWorld"), "(doform \"HelloWorld\")");
    assert_eq!(ok("CLEAR EVENTS"), "(clear-events)");
    assert_eq!(ok("MESSAGEBOX('HelloWorld 1.0')"), "(expr MESSAGEBOX(\"HelloWorld 1.0\"))");
}

#[test]
fn declarations() {
    assert_eq!(ok("LOCAL a, b, c"), "(local A B C)");
    assert_eq!(ok("LOCAL a AS Integer, b AS Object OF \"x.vcx\", c"), "(local A B C)");
    assert_eq!(ok("LOCAL ARRAY a[3], b(2, 2)"), "(local A[3] B[2, 2])");
    assert_eq!(ok("LOCAL a(3)"), "(local A[3])");
    assert_eq!(ok("PRIVATE x, y"), "(private X Y)");
    assert_eq!(ok("PUBLIC gcName, gaList[10]"), "(public GCNAME GALIST[10])");
    assert_eq!(ok("PUBLIC ARRAY gaList[2]"), "(public GALIST[2])");
    // PRIVATE only hides what the caller owns, so ARRAY needs no size and any size given is
    // thrown away: real VFP leaves the name undefined until something DIMENSIONs it
    assert_eq!(ok("PRIVATE ARRAY paDBFields"), "(private PADBFIELDS)");
    assert_eq!(ok("PRIVATE ARRAY paA[2], paB"), "(private PAA[2] PAB)");
    assert_eq!(ok("DIMENSION arr[3, 4], other(n + 1)"), "(dimension ARR[3, 4] OTHER[(+ N 1)])");
    assert_eq!(ok("DECLARE arr(3)"), "(dimension ARR[3])");
    assert_eq!(ok("DIME arr[1]"), "(dimension ARR[1])");
    assert_eq!(ok("LPARAMETERS a, b AS String"), "(lparameters A B)");
    assert_eq!(ok("LPARA a"), "(lparameters A)");
    assert_eq!(ok("PARAMETERS tcName, tnAge"), "(parameters TCNAME TNAGE)");
}

#[test]
fn private_all_warns_and_is_skipped() {
    let out = parse_program("PRIVATE ALL LIKE x*\nx = 1");
    assert_eq!(messages(&out.diagnostics), vec!["PRIVATE ALL is not supported"]);
    assert_eq!(out.diagnostics[0].severity, Severity::Warning);
    assert_eq!(dump(&out.program), "(= X 1)");
}

#[test]
fn assignments_and_expression_statements() {
    assert_eq!(ok("x = 1"), "(= X 1)");
    assert_eq!(ok("x=1"), "(= X 1)");
    assert_eq!(ok("obj.prop = 2"), "(= OBJ.PROP 2)");
    assert_eq!(ok("arr[1, 2] = 3"), "(= ARR[1, 2] 3)");
    assert_eq!(ok("arr(1) = 3"), "(= ARR(1) 3)");
    assert_eq!(ok("THIS.Value = .T."), "(= THIS.VALUE .T.)");
    assert_eq!(ok("=MESSAGEBOX(\"hi\")"), "(expr MESSAGEBOX(\"hi\"))");
    assert_eq!(ok("= x + 1"), "(expr (+ X 1))");
    assert_eq!(ok("foo(1, 2)"), "(expr FOO(1, 2))");
    assert_eq!(ok("obj.Refresh()"), "(expr OBJ.REFRESH())");
    assert_eq!(ok("STORE 0 TO a, b, obj.c"), "(store 0 A, B, OBJ.C)");
    assert_eq!(ok("x = y = z"), "(= X (= Y Z))");
    assert_eq!(ok("total = 5\ncount = count + 1"), "(= TOTAL 5)\n(= COUNT (+ COUNT 1))");
}

#[test]
fn print_statements() {
    assert_eq!(ok("? \"a\", b"), "(? \"a\", B)");
    assert_eq!(ok("?"), "(?)");
    assert_eq!(ok("?? x"), "(?? X)");
}

#[test]
fn if_else() {
    assert_eq!(ok("IF a > 1\n  x = 1\nELSE\n  x = 2\nENDIF"), "(if (> A 1) (block (= X 1)) (block (= X 2)))");
    assert_eq!(ok("IF a THEN\n  x = 1\nENDIF"), "(if A (block (= X 1)))");
    assert_eq!(ok("if a\nendi"), "(if A (block))");
    assert_eq!(
        ok("IF a\n  IF b\n    x = 1\n  ELSE\n    x = 2\n  ENDIF\nELSE\n  IF c\n    x = 3\n  ENDIF\nENDIF"),
        "(if A (block (if B (block (= X 1)) (block (= X 2)))) (block (if C (block (= X 3)))))"
    );
}

#[test]
fn do_case() {
    let src = "DO CASE\n\nCASE x = 1\n  y = 1\nCASE x = 2\n  y = 2\n  IF y > 0\n    z = 1\n  ENDIF\nOTHERWISE\n  y = 0\nENDCASE";
    assert_eq!(
        ok(src),
        "(docase (case (= X 1) (block (= Y 1))) (case (= X 2) (block (= Y 2) (if (> Y 0) (block (= Z 1))))) (otherwise (block (= Y 0))))"
    );
    assert_eq!(ok("DO CASE\nCASE a\nENDCASE"), "(docase (case A (block)))");
    assert_eq!(ok("DO CASE\nOTHE\n x = 1\nENDC"), "(docase (otherwise (block (= X 1))))");
}

#[test]
fn do_case_rejects_statements_before_first_case() {
    let out = parse_program("DO CASE\n  x = 1\nCASE a\nENDCASE");
    assert_eq!(messages(&out.diagnostics), vec!["Statements are not allowed between DO CASE and the first CASE"]);
    assert_eq!(out.diagnostics[0].line, 2);
    assert_eq!(dump(&out.program), "(docase (case A (block)))");
}

#[test]
fn loops() {
    assert_eq!(
        ok("DO WHILE x < 10\n  x = x + 1\n  IF x = 5\n    EXIT\n  ENDIF\n  LOOP\nENDDO"),
        "(while (< X 10) (block (= X (+ X 1)) (if (= X 5) (block (exit))) (loop)))"
    );
    assert_eq!(ok("FOR i = 1 TO 10\n  ? i\nENDFOR"), "(for I 1 10 (block (? I)))");
    assert_eq!(ok("FOR i = 10 TO 1 STEP -1\n  ? i\nNEXT i"), "(for I 10 1 (- 1) (block (? I)))");
    assert_eq!(ok("FOR i = 1 TO 3\nNEXT"), "(for I 1 3 (block))");
    assert_eq!(
        ok("FOR EACH o IN THISFORM.Controls\n  o.Visible = .F.\nENDFOR"),
        "(foreach O THISFORM.CONTROLS (block (= O.VISIBLE .F.)))"
    );
    assert_eq!(ok("FOR EACH o IN coll FOXOBJECT\nNEXT o"), "(foreach O COLL (block))");
    // `m.` on the loop variable, any expression to walk, and the clauses in either order
    assert_eq!(ok("FOR EACH m.loForm IN _SCREEN.Forms  FOXOBJECT\nENDFOR"), "(foreach LOFORM _SCREEN.FORMS (block))");
    assert_eq!(ok("FOR EACH m.loFX IN THIS.FXs AS Object FOXOBJECT\nENDFOR"), "(foreach LOFX THIS.FXS (block))");
    assert_eq!(ok("FOR EACH m.o IN coll FOXOBJECT AS Object\nENDFOR"), "(foreach O COLL (block))");
    assert_eq!(
        ok("FOR i = 1 TO 2\n  FOR j = 1 TO 2\n    DO CASE\n    CASE i = j\n      ? i\n    ENDCASE\n  ENDFOR\nENDFOR"),
        "(for I 1 2 (block (for J 1 2 (block (docase (case (= I J) (block (? I))))))))"
    );
}

#[test]
fn return_forms() {
    assert_eq!(ok("RETURN"), "(return)");
    assert_eq!(ok("RETURN x + 1"), "(return (+ X 1))");
    assert_eq!(ok("RETU .T."), "(return .T.)");
    assert_eq!(ok("RETURN TO MASTER"), "(return-to master)");
    assert_eq!(ok("RETURN TO Outer"), "(return-to OUTER)");
}

#[test]
fn do_program() {
    assert_eq!(ok("DO main"), "(do main)");
    assert_eq!(ok("DO main.prg"), "(do main.prg)");
    assert_eq!(ok("DO setup WITH 1, \"x\", @ref"), "(do setup (with 1, \"x\", @REF))");
    assert_eq!(ok("DO setup WITH a,,b"), "(do setup (with A, _, B))");
    assert_eq!(ok("DO helper IN lib.prg"), "(do helper (in \"LIB.PRG\"))");
    assert_eq!(ok("DO helper WITH 1 IN lib"), "(do helper (with 1) (in \"LIB\"))");
    assert_eq!(ok("DO (cProg)"), "(do-expr CPROG)");
    assert_eq!(ok("DO (cProg + \".prg\") WITH 1"), "(do-expr (+ CPROG \".prg\") (with 1))");
}

#[test]
fn do_form() {
    assert_eq!(ok("DO FORM HelloWorld.scx"), "(doform \"HelloWorld.scx\")");
    assert_eq!(ok("DO FORM (cName)"), "(doform CNAME)");
    assert_eq!(ok("DO FORM \"other.fxf\""), "(doform \"other.fxf\")");
    assert_eq!(
        ok("DO FORM Editor NAME oEditor LINKED WITH 1, 2 TO nResult NOSHOW"),
        "(doform \"Editor\" (name OEDITOR) linked (with 1, 2) (to NRESULT) noshow)"
    );
    assert_eq!(ok("DO FORM Editor NOSHOW NAME oForm"), "(doform \"Editor\" (name OFORM) noshow)");
    assert_eq!(ok("DO FORM Editor NOREAD"), "(doform \"Editor\")");
    // the form is named by a name expression, so a call that works one out will do, and
    // LINKED is written wherever the line has room for it
    assert_eq!(ok("DO FORM locfile(lcFile)"), "(doform LOCFILE(LCFILE))");
    assert_eq!(
        ok("DO FORM LOCFILE(\"ParamAsk.scx\") WITH cP1, nP2 TO nRet"),
        "(doform LOCFILE(\"ParamAsk.scx\") (with CP1, NP2) (to NRET))"
    );
    assert_eq!(
        ok("DO FORM Multi NAME THISFORM.aForms[nInstance] WITH 1 LINKED"),
        "(doform \"Multi\" (name THISFORM.AFORMS[NINSTANCE]) linked (with 1))"
    );
    // and where the form object is kept may be named by an expression as well
    assert_eq!(
        ok("DO FORM (HOME(2) + \"x.scx\") NAME (lcFormName)"),
        "(doform (+ HOME(2) \"x.scx\") (name (by-name LCFORMNAME)))"
    );
    // Written plainly the form is a file name and not a name, so a word no identifier could be
    // is one: measured, DO FORM 1_many opens 1_many.scx, one of the forms Visual FoxPro ships.
    assert_eq!(ok("DO FORM 1_many NOSHOW"), "(doform \"1_many\" noshow)");
    // a relative path with an extension comes through as it stands
    assert_eq!(ok("DO FORM .\\sub\\paramask.scx"), "(doform \".\\\\sub\\\\paramask.scx\")");
    // and a variable is its own name rather than what it holds: measured, with cName holding
    // "paramask", DO FORM cName says file 'cname.scx' does not exist.
    assert_eq!(ok("DO FORM cName WITH 1"), "(doform \"cName\" (with 1))");
}

/// Almost anywhere a name is expected, a parenthesised expression or a quoted name will do.
#[test]
fn name_expressions() {
    // the work area an IN clause names
    assert_eq!(ok("REPLACE town WITH x IN (lcAlias)"), "(replace in (LCALIAS) current (TOWN X))");
    assert_eq!(ok("REPLACE town WITH x IN \"people\""), "(replace in PEOPLE current (TOWN X))");
    assert_eq!(ok("APPEND BLANK IN (m.cPapa)"), "(append in (M.CPAPA))");
    assert_eq!(ok("APPEND BLANK IN 0"), "(append in (0))");
    assert_eq!(ok("ZAP IN (lcAlias)"), "(zap in (LCALIAS))");
    assert_eq!(ok("UNLOCK RECORD RECNO(lcAlias) IN (lcAlias)"), "(unlock record RECNO(LCALIAS) in (LCALIAS))");
    assert_eq!(ok("SET ORDER TO 0 IN (lcAlias)"), "(set-order 0)");
    // the field a REPLACE writes, and the columns of a table being made or altered
    assert_eq!(ok("REPLACE (lcField) WITH x"), "(replace current ((LCFIELD) X))");
    assert_eq!(
        ok("CREATE TABLE (THIS.cOutFile) FREE ((THIS.cDefNewField) g)"),
        "(create-table THIS.COUTFILE ((THIS.CDEFNEWFIELD) G(4,0)))"
    );
    assert_eq!(
        ok("ALTER TABLE (m.tcMeta) ADD COLUMN (field(liIndex)) M"),
        "(alter-table M.TCMETA (add ((FIELD(LIINDEX)) M(4,0))))"
    );
    // the cursor a query is gathered into, and the table it reads
    assert_eq!(
        ok("SELECT * FROM parts INTO CURSOR (m.lcPEAlias)"),
        "(select (*) (from parts:PARTS) (into-cursor (M.LCPEALIAS)))"
    );
    assert_eq!(ok("SELECT * FROM \"parts\" INTO CURSOR c"), "(select (*) (from parts:PARTS) (into-cursor C))");
    // the alias a table is opened under
    assert_eq!(
        ok("USE (THIS.cDBFName) AGAIN SHARED ALIAS \"keywords\""),
        "(use THIS.CDBFNAME (alias KEYWORDS))"
    );
    // a variable being declared, and one being stored to - which may name a property
    assert_eq!(ok("PRIVATE (lcVariable)"), "(private (LCVARIABLE))");
    assert_eq!(ok("STORE 1 TO (lcName)"), "(store 1 (by-name LCNAME))");
    assert_eq!(
        ok("STORE lnValue TO (\"THIS.oObject.\" + lcProperty)"),
        "(store LNVALUE (by-name (+ \"THIS.oObject.\" LCPROPERTY)))"
    );
    assert_eq!(ok("STORE 1 TO a, (b)"), "(store 1 A, (by-name B))");
}

#[test]
fn wait_window() {
    assert_eq!(ok("WAIT WINDOW \"ok\""), "(wait \"ok\")");
    assert_eq!(ok("WAIT"), "(wait)");
    assert_eq!(ok("WAIT WINDOW NOWAIT"), "(wait nowait)");
    assert_eq!(ok("WAIT WINDOW \"ok\" NOWAIT TIMEOUT 2"), "(wait \"ok\" nowait (timeout 2))");
    assert_eq!(ok("WAIT WINDOW \"ok\" AT 1, 2 NOCLEAR TO cKey"), "(wait \"ok\" (to CKEY))");
    assert_eq!(ok("WAIT CLEAR"), "(wait clear)");
    assert_eq!(ok("WAIT \"press\" TO cKey"), "(wait \"press\" (to CKEY))");
    assert_eq!(ok("WAIT WIND cMsg TIMEOUT n NOWAIT"), "(wait CMSG nowait (timeout N))");
}

#[test]
fn read_and_clear() {
    assert_eq!(ok("READ EVENTS"), "(read-events)");
    assert_eq!(ok("CLEAR EVENTS"), "(clear-events)");
    // READ on its own lets the user into the fields an `@ ... GET` put up
    assert_eq!(ok("READ"), "(window Read)");
    // CLEAR on its own wipes the character screen, CLEAR ALL lets go of what the program made,
    // and CLEAR TYPEAHEAD empties the buffer KEYBOARD fills
    let out = parse_program("CLEAR\nCLEAR ALL\nCLEAR MEMORY\nCLEAR TYPEAHEAD\nx = 1");
    assert_eq!(messages(&out.diagnostics), Vec::<String>::new());
    assert_eq!(dump(&out.program), "(window ClearScreen)
(clear all)
(clear memory)
(keyboard)
(= X 1)");
}

#[test]
fn release() {
    assert_eq!(ok("RELEASE a, b"), "(release A, B)");
    assert_eq!(ok("RELEASE THISFORM"), "(release THISFORM)");
    assert_eq!(ok("RELEASE oForm.oChild"), "(release OFORM.OCHILD)");
    assert_eq!(ok("RELEASE ALL"), "(release-all)");
    let out = parse_program("RELEASE ALL LIKE x*");
    assert_eq!(out.diagnostics.len(), 1);
    assert_eq!(out.diagnostics[0].severity, Severity::Warning);
    assert_eq!(dump(&out.program), "(release-all)");
    let out = parse_program("RELEASE WINDOW foo\nx = 1");
    assert_eq!(messages(&out.diagnostics), Vec::<String>::new());
    assert_eq!(dump(&out.program), "(window Release)\n(= X 1)");
    let out = parse_program("RELEASE BAR 1 OF popfile\nx = 1");
    assert_eq!(messages(&out.diagnostics), Vec::<String>::new());
    assert_eq!(dump(&out.program), "(menu ReleaseBar)\n(= X 1)");
    let out = parse_program("RELEASE PAD ALL OF mainbar");
    assert_eq!(dump(&out.program), "(menu ReleasePad)");
    // the ones with nothing of that kind to release still say so
    let out = parse_program("RELEASE PROCEDURE lib\nx = 1");
    assert!(out.diagnostics.iter().all(|d| d.severity == Severity::Warning));
}

#[test]
fn quit_and_cancel() {
    assert_eq!(ok("QUIT\nCANCEL"), "(quit)\n(cancel)");
}

#[test]
fn with_block() {
    assert_eq!(
        ok(
            "WITH THISFORM.pgfMain\n  .Caption = \"x\"\n  .Refresh()\n  .Pages(1).Caption = .Caption\n  x = .Width * 2\nENDWITH"
        ),
        "(with THISFORM.PGFMAIN (block (= <with>.CAPTION \"x\") (expr <with>.REFRESH()) (= <with>.PAGES(1).CAPTION <with>.CAPTION) (= X (* <with>.WIDTH 2))))"
    );
}

#[test]
fn set_statements() {
    assert_eq!(ok("SET EXACT ON"), "(set EXACT on)");
    assert_eq!(ok("SET TALK OFF"), "(set TALK off)");
    assert_eq!(ok("SET DECIMALS TO 4"), "(set DECIMALS (to 4))");
    assert_eq!(ok("SET DATE TO AMERICAN"), "(set DATE (word \"AMERICAN\"))");
    assert_eq!(ok("SET DATE AMERICAN"), "(set DATE (word \"AMERICAN\"))");
    // the flag, then the alias, then the files: what the CLASSLIB arm reads its arguments as
    assert_eq!(ok("SET CLASSLIB TO x ADDITIVE"), "(set CLASSLIB (to .T., \"\", \"x\"))");
    assert_eq!(ok("SET CLASSLIB TO ..\\solution"), "(set CLASSLIB (to .F., \"\", \"..\\\\solution\"))");
    assert_eq!(ok("SET CLASSLIB TO a, b ALIAS two"), "(set CLASSLIB (to .F., \"two\", \"a\", \"b\"))");
    assert_eq!(ok("SET CLASSLIB TO (lcFile)"), "(set CLASSLIB (to .F., \"\", LCFILE))");
    assert_eq!(ok("SET CLASSLIB TO"), "(set CLASSLIB (to .F., \"\"))");
    // SET PATH TO names folders rather than expressions, so the line is kept as it was written.
    // A line that opens with a quote or a bracket is the exception and is read as an expression,
    // and what follows it is dropped: measured, `SET PATH TO 'a' , 'b'` leaves SET("PATH")
    // answering `A`.
    assert_eq!(ok("SET PATH TO a,b"), "(set PATH (word \"a,b\"))");
    assert_eq!(ok("SET PATH TO b ADDITIVE"), "(set PATH (word \"b ADDITIVE\"))");
    assert_eq!(ok("SET PATH TO \"a;b\", \"c\""), "(set PATH (to \"a;b\"))");
    assert_eq!(ok("SET PATH TO (lcPath)"), "(set PATH (to LCPATH))");
    assert_eq!(ok("SET PATH TO"), "(set PATH (word \"\"))");
    assert_eq!(ok("SET PROCEDURE TO"), "(set PROCEDURE (word \"\"))");
    assert_eq!(ok("SET STATUS BAR ON"), "(set STATUS BAR on)");
    assert_eq!(ok("SET SYSMENU TO DEFAULT"), "(menu SetSysMenu)");
    assert_eq!(ok("SET STEP"), "(set STEP (word \"\"))");
    assert_eq!(ok("SET MESSAGE TO 2 LEFT"), "(set MESSAGE (word \"2 LEFT\"))");
    assert_eq!(ok("SET MESSAGE TO \"Ready\""), "(menu SetMessage)");
}

#[test]
fn try_catch_finally() {
    assert_eq!(
        ok("TRY\n  x = 1 / 0\nCATCH TO oErr WHEN oErr.ErrorNo = 1307\n  ? oErr.Message\nFINALLY\n  x = 0\nENDTRY"),
        "(try (block (= X (/ 1 0))) (catch (to OERR) (when (= OERR.ERRORNO 1307)) (block (? OERR.MESSAGE))) (finally (block (= X 0))))"
    );
    assert_eq!(ok("TRY\nCATCH\n  THROW\nENDTRY"), "(try (block) (catch (block (throw))))");
    assert_eq!(ok("TRY\n  THROW \"boom\"\nFINALLY\nENDTRY"), "(try (block (throw \"boom\")) (finally (block)))");
    // several CATCH clauses on one TRY, each tried in turn
    assert_eq!(
        ok("TRY\nCATCH TO e WHEN e.ErrorNo = 12\n  ? 1\nCATCH TO e\n  ? 2\nENDTRY"),
        "(try (block) (catch (to E) (when (= E.ERRORNO 12)) (block (? 1))) (catch (to E) (block (? 2))))"
    );
    // `m.` says the caught error goes into a memory variable, which is where it was going
    assert_eq!(ok("TRY\nCATCH TO m.oError\nENDTRY"), "(try (block) (catch (to OERROR) (block)))");
}

#[test]
fn bracket_strings_survive_in_command_position() {
    // a command clause wants an expression, so `[...]` there is a string, spaces or not
    assert_eq!(ok("WAIT WINDOW [please wait]"), "(wait \"please wait\")");
    assert_eq!(ok("? [hello]"), "(? \"hello\")");
    assert_eq!(ok("x = [hello]"), "(= X \"hello\")");
    // but a declaration's dimensions are subscripts even with a space before the bracket
    assert_eq!(ok("DIMENSION a [3]"), "(dimension A[3])");
    assert_eq!(ok("LOCAL ARRAY b [2, 2]"), "(local B[2, 2])");
}

#[test]
fn erase_and_external() {
    assert_eq!(ok("ERASE Sample.txt"), "(erase \"Sample.txt\")");
    assert_eq!(ok("ERASE (m.lcFile)"), "(erase M.LCFILE)");
    // EXTERNAL only tells the project manager what a program references; it does not run
    let out = parse_program("EXTERNAL ARRAY aList\n? 1");
    assert_eq!(messages(&out.diagnostics), Vec::<String>::new());
    assert_eq!(out.program.body.stmts.len(), 1);
}

#[test]
fn bare_member_is_a_method_call() {
    assert_eq!(ok("THIS.ResizeChars"), "(expr THIS.RESIZECHARS())");
    assert_eq!(ok("RETURN @This.aRGB"), "(return THIS.ARGB)");
}

#[test]
fn nodefault_and_dodefault() {
    assert_eq!(ok("NODEFAULT"), "(nodefault)");
    assert_eq!(ok("DODEFAULT()"), "(dodefault )");
    assert_eq!(ok("DODEFAULT(1, x)"), "(dodefault 1, X)");
    assert_eq!(ok("DODEFAULT"), "(dodefault )");
    assert_eq!(ok("x = DODEFAULT()"), "(= X DODEFAULT())");
}

#[test]
fn procedures_and_functions() {
    let src = "x = 1\nDO setup\n\nPROCEDURE setup\n  LPARAMETERS a, b\n  ? a\nENDPROC\n\nFUNCTION add(a, b AS Integer)\n  RETURN a + b\nENDFUNC\nPROCEDURE noend\n  ? 1\nPROC other\n  ? 2\n";
    let out = parse_program(src);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(
        dump(&out.program),
        "(= X 1)\n(do setup)\n(procedure SETUP ())\n  (lparameters A B)\n  (? A)\n(function ADD (A B))\n  (return (+ A B))\n(procedure NOEND ())\n  (? 1)\n(procedure OTHER ())\n  (? 2)"
    );
    assert_eq!(out.program.procs[0].kind, ProcKind::Procedure);
    assert_eq!(out.program.procs[1].kind, ProcKind::Function);
    assert_eq!(out.program.procs[0].line, 4);
    assert_eq!(out.program.procs[1].line, 9);
    assert_eq!(out.program.procs[1].name.text, "add");
}

#[test]
fn procedure_with_empty_parens_and_no_body() {
    assert_eq!(ok("PROCEDURE a()\nPROCEDURE b\nENDPROC"), "(procedure A ())\n(procedure B ())");
}

#[test]
fn define_substitution() {
    assert_eq!(
        ok(
            "#DEFINE MAX_ROWS 10\n#define GREETING \"hi\" + \" there\"\nx = MAX_ROWS + 1\n? greeting\n#UNDEF MAX_ROWS\ny = max_rows"
        ),
        "(directive \"#DEFINE MAX_ROWS 10\")\n(directive \"#define GREETING \\\"hi\\\" + \\\" there\\\"\")\n(= X (+ 10 1))\n(? (+ \"hi\" \" there\"))\n(directive \"#UNDEF MAX_ROWS\")\n(= Y MAX_ROWS)"
    );
}

#[test]
fn define_keeps_usage_span_and_line() {
    let out = parse_program("#DEFINE ONE 1\n\nx = ONE");
    let StmtKind::Assign { value, .. } = &out.program.body.stmts[1].kind else { panic!() };
    assert_eq!((value.span.start, value.span.end), (19, 22));
    assert_eq!(out.program.body.stmts[1].line, 3);
}

#[test]
fn a_conditional_keeps_the_half_the_condition_chooses() {
    let out = parse_program("#INCLUDE \"foxpro.h\"\n#IF .T.\nx = 1\n#ELSE\nx = 2\n#ENDIF");
    // a header the caller did not hand over is a warning by name, and the rest is read as
    // usual: the true half is kept and the other is not
    assert_eq!(messages(&out.diagnostics), vec!["#INCLUDE foxpro.h: the file was not found"]);
    assert!(out.diagnostics.iter().all(|d| d.severity == Severity::Warning));
    assert_eq!(
        dump(&out.program),
        "(directive \"#INCLUDE \\\"foxpro.h\\\"\")\n(directive \"#IF .T.\")\n(= X 1)\n(directive \"#ELSE\")"
    );
}

#[test]
fn text_blocks() {
    assert_eq!(
        ok("TEXT TO cHtml TEXTMERGE NOSHOW\n<h1><<title>></h1>\n  line 2\nENDTEXT\n? cHtml"),
        "(text (to CHTML) textmerge noshow \"<h1><<title>></h1>\\n  line 2\")\n(? CHTML)"
    );
    assert_eq!(ok("TEXT TO x ADDITIVE\nabc\nENDTEXT"), "(text (to X) additive \"abc\")");
    assert_eq!(ok("TEXT\nplain\nENDTEXT"), "(text \"plain\")");
    assert_eq!(ok("TEXT TO o.prop NOSHOW FLAGS 1 PRETEXT 2\nz\nENDTEXT"), "(text (to O.PROP) noshow \"z\")");
    let out = parse_program("x = 1\nTEXT TO y\nabc");
    assert_eq!(messages(&out.diagnostics), vec!["TEXT without matching ENDTEXT"]);
    assert_eq!(out.diagnostics[0].line, 2);
}

#[test]
fn backslash_lines_are_one_line_of_text_merge_output() {
    // the rest of the line is text, whatever is written in it, and nothing after the marker goes
    assert_eq!(ok("\\ <<cName>>, \"quoted\""), "(text-line \\ \" <<cName>>, \\\"quoted\\\"\")");
    assert_eq!(ok("\\\\same line"), "(text-line \\\\ \"same line\")");
    assert_eq!(ok("\\"), "(text-line \\ \"\")");
}

#[test]
fn set_textmerge_carries_its_words_and_what_it_named() {
    assert_eq!(ok("SET TEXTMERGE ON NOSHOW"), "(set TEXTMERGE (to \"ON NOSHOW\"))");
    assert_eq!(
        ok("SET TEXTMERGE TO MEMVAR m.lcOut ADDITIVE NOSHOW"),
        "(set TEXTMERGE (to \"TO MEMVAR ADDITIVE NOSHOW\", \"LCOUT\"))"
    );
    assert_eq!(ok("SET TEXTMERGE TO out.txt"), "(set TEXTMERGE (to \"TO\", \"out.txt\"))");
    assert_eq!(ok("SET TEXTMERGE TO"), "(set TEXTMERGE (to \"TO\"))");
    assert_eq!(
        ok("SET TEXTMERGE DELIMITERS TO \"{{\", \"}}\""),
        "(set TEXTMERGE DELIMITERS (to \"DELIMITERS\", \"{{\", \"}}\"))"
    );
}

#[test]
fn on_error_and_on_key_label() {
    assert_eq!(ok("ON ERROR DO handler WITH ERROR(), MESSAGE()"), "(onerror \"DO handler WITH ERROR(), MESSAGE()\")");
    assert_eq!(ok("ON ERROR"), "(onerror)");
    assert_eq!(ok("ON ERROR ?\"oops\""), "(onerror \"?\\\"oops\\\"\")");
    assert_eq!(ok("ON KEY LABEL F5 DO refresh"), "(onkey \"F5\" \"DO refresh\")");
    assert_eq!(ok("ON KEY LABEL CTRL+F5 =refresh()"), "(onkey \"CTRL+F5\" \"=refresh()\")");
    assert_eq!(ok("ON KEY LABEL ALT+X"), "(onkey \"ALT+X\")");
    // the rest of the family hangs a command off something that may happen, and each one
    // says which of them it is
    assert_eq!(ok("ON SHUTDOWN QUIT"), "(on \"SHUTDOWN\" \"QUIT\")");
    assert_eq!(ok("ON ESCAPE"), "(on \"ESCAPE\")");
    assert_eq!(ok("ON PAGE AT LINE 55 DO header"), "(on \"PAGE 55\" \"DO header\")");
    assert_eq!(ok("ON KEY = 27 ?\"esc\""), "(on \"KEY 27\" \"?\\\"esc\\\"\")");
    // one that names nothing it could hang off is still only a warning
    let out = parse_program("ON APLABOUT DO x
? \"after\"");
    assert!(out.diagnostics.iter().all(|d| d.severity == Severity::Warning));
}

#[test]
fn macro_line() {
    // a line whose shape is not known until the macro is put into it is kept as it was
    // written and read at the moment it runs, which is what VFP does with every line
    assert_eq!(ok("&cmd"), "(macro-text \"&cmd\")");
    assert_eq!(ok("&cmd."), "(macro-text \"&cmd.\")");
    assert_eq!(ok("&cmd + 1"), "(macro-text \"&cmd + 1\")");
    assert_eq!(ok("&cRef = .NULL."), "(macro-text \"&cRef = .NULL.\")");
    // a macro standing for a whole clause list keeps the block it opens with it
    assert_eq!(ok("SCAN &lcScope\n? 1\nENDSCAN"), "(macro-text \"SCAN &lcScope\\n? 1\\nENDSCAN\")");
    assert_eq!(ok("SET ORDER TO (lcTag) &lcAlias ASCENDING"), "(macro-text \"SET ORDER TO (lcTag) &lcAlias ASCENDING\")");
    // a date built out of macros is not a bad constant, it is one whose text arrives later
    assert_eq!(ok("x = {^&lcYear./01/01}"), "(macro-text \"x = {^&lcYear./01/01}\")");
    // where a macro stands for a value the expression reads it as it stands
    assert_eq!(ok("x = &cName"), "(= X &CNAME)");
    assert_eq!(ok("x = &laNames[m.i]"), "(= X &LANAMES[M.I])");
    assert_eq!(ok("x = o.&cName"), "(= X (O.&CNAME))");
    assert_eq!(ok("x = o.&laObjs[m.i]..Class"), "(= X (O.&LAOBJS[M.I]).CLASS)");
    assert_eq!(ok("WITH o\n.&cName = 1\nENDWITH"), "(with O (block (= (<with>.&CNAME) 1)))");
}

#[test]
fn keyword_abbreviations() {
    assert_eq!(ok("LOCA x\nPRIV y\nPUBL z"), "(local X)\n(private Y)\n(public Z)");
    assert_eq!(ok("DO WHIL .T.\n  EXIT\nENDD"), "(while .T. (block (exit)))");
    assert_eq!(ok("FOR i = 1 TO 2\nENDF"), "(for I 1 2 (block))");
    assert_eq!(ok("FUNC f\nENDF"), "(function F ())");
    assert_eq!(ok("WAIT WIND \"x\" NOWA TIME 1"), "(wait \"x\" nowait (timeout 1))");
    assert_eq!(ok("RELE x"), "(release X)");
    assert_eq!(ok("CLEA EVEN\nREAD EVEN"), "(clear-events)\n(read-events)");
    assert_eq!(ok("TRY\nCATC\nFINA\nENDT"), "(try (block) (catch (block)) (finally (block)))");
}

#[test]
fn statement_lines_and_spans() {
    let src = "x = 1\n\n  IF x ;\n     = 1\n    y = 2\n  ENDIF\n";
    let out = parse_program(src);
    assert!(out.diagnostics.is_empty());
    let s = &out.program.body.stmts;
    assert_eq!(s[0].line, 1);
    assert_eq!((s[0].span.start, s[0].span.end), (0, 5));
    assert_eq!(s[1].line, 3);
    assert_eq!((s[1].span.start, s[1].span.end), (9, src.len() - 1));
    let StmtKind::If { then, .. } = &s[1].kind else { panic!() };
    assert_eq!(then.stmts[0].line, 5);
}

#[test]
fn comments_and_continuations_in_programs() {
    assert_eq!(
        ok("* header\nNOTE more\nx = 1 && trailing\ny = 1 + ;\n    2 ; && why not\n  + 3\n"),
        "(= X 1)\n(= Y (+ (+ 1 2) 3))"
    );
}

#[test]
fn parse_method_allows_return_and_rejects_procedure() {
    let out = parse_method("IF x\n  RETURN .F.\nENDIF\nRETURN .T.");
    assert!(out.diagnostics.is_empty());
    let out = parse_method("PROCEDURE foo\nENDPROC");
    assert_eq!(
        messages(&out.diagnostics),
        vec!["PROCEDURE declarations are not allowed inside a method", "ENDPROC without matching PROCEDURE"]
    );
}

#[test]
fn full_program_mix() {
    let src = r#"
* HelloWorld main program
#DEFINE APP_NAME "Hello"
SET EXACT ON
SET DECIMALS TO 4
SET DATE TO AMERICAN
ON ERROR DO handler
LOCAL i, done
done = .F.
FOR i = 1 TO 3
  DO CASE
  CASE i = 1
    WAIT WINDOW APP_NAME NOWAIT
  CASE i = 2 .AND. NOT done
    ? "two"
  OTHERWISE
    done = .T.
  ENDCASE
ENDFOR
TEXT TO cOut TEXTMERGE
  Hello <<APP_NAME>>
ENDTEXT
DO FORM HelloWorld
READ EVENTS

PROCEDURE handler
  LPARAMETERS nErr
  ? nErr
ENDPROC
"#;
    let out = parse_program(src);
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(
        dump(&out.program),
        "(directive \"#DEFINE APP_NAME \\\"Hello\\\"\")\n\
         (set EXACT on)\n\
         (set DECIMALS (to 4))\n\
         (set DATE (word \"AMERICAN\"))\n\
         (onerror \"DO handler\")\n\
         (local I DONE)\n\
         (= DONE .F.)\n\
         (for I 1 3 (block (docase (case (= I 1) (block (wait \"Hello\" nowait))) (case (AND (= I 2) (! DONE)) (block (? \"two\"))) (otherwise (block (= DONE .T.))))))\n\
         (text (to COUT) textmerge \"  Hello <<APP_NAME>>\")\n\
         (doform \"HelloWorld\")\n\
         (read-events)\n\
         (procedure HANDLER ())\n  (lparameters NERR)\n  (? NERR)"
    );
}

#[test]
fn a_declaration_list_may_be_closed_with_a_paren() {
    // Visual FoxPro reads a declaration's names as if the list were optionally parenthesised.
    // Measured: `LPARAMETERS ta, tb)` and `LOCAL x, y)` compile with no error file; a word after
    // the list and a `)` where a name should be are both syntax errors. The Foundation Classes
    // ship `_reportlistener.vcx sendfx`, whose LPARAMETERS ends with a stray `)`.
    for src in ["LPARAMETERS ta, tb)\n", "PARAMETERS ta, tb)\n", "LOCAL x, y)\n", "PRIVATE x)\n"] {
        let out = parse_program(src);
        assert!(out.diagnostics.is_empty(), "{src:?}: {:#?}", out.diagnostics);
    }
    assert!(!parse_program("LPARAMETERS ta, tb junk\n").diagnostics.is_empty());
    assert!(!parse_program("LPARAMETERS ta, tb, )\n").diagnostics.is_empty());
}
