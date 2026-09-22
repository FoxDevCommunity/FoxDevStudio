* p0-behaviour: what REPORT FORM puts where, under SET REPORTBEHAVIOR 80 and 90.
*
* A probe, not a golden: run it with
*   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-behaviour.prg
* The wrapper captures the ? channel with SET ALTERNATE, so what a bare REPORT FORM sends to the
* console is what the printed output shows between the BEGIN/END markers. Each TO FILE result
* is an fdv-*.txt file of its own. Statements that print come last and are limited to one page
* with RANGE 1,1, in case a print job stops the run; measured, they return, and it is PREVIEW
* that stops to ask, which the harness answers with ESC and records as having asked.
ON ERROR DO fdvErr WITH ERROR(), MESSAGE(), LINENO(), PROGRAM()
SET SAFETY OFF
SET TALK OFF
PUBLIC gcRun, gcContext, gnErr
gcRun = ADDBS(SYS(5) + SYS(2003))
gcContext = "start"
gnErr = 0
LOCAL cFrx, cRep, cRaw, i, cHex, ox, oDE, nBehaviour
LOCAL ARRAY aRaw[1]
cRep = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\"
cFrx = cRep + "percent.frx"
fdvPrinter()

? "SET(REPORTBEHAVIOR) at start:", SET("REPORTBEHAVIOR"), VARTYPE(SET("REPORTBEHAVIOR"))
? "_REPORTOUTPUT:", _REPORTOUTPUT
? "_REPORTPREVIEW:", _REPORTPREVIEW
? "_REPORTBUILDER:", _REPORTBUILDER
? "_PAGENO before anything:", _PAGENO, "_PAGETOTAL:", _PAGETOTAL
SET REPORTBEHAVIOR 90
? "after SET REPORTBEHAVIOR 90:", SET("REPORTBEHAVIOR")
SET REPORTBEHAVIOR 80
? "after SET REPORTBEHAVIOR 80:", SET("REPORTBEHAVIOR")

FOR nBehaviour = 80 TO 90 STEP 10
  SET REPORTBEHAVIOR (nBehaviour)
  gcContext = "behaviour " + TRANSFORM(nBehaviour)
  ? "===== REPORTBEHAVIOR", nBehaviour

  * the ASCII text, once per clause, each to its own file
  _PAGENO = 1
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-ascii.txt") ASCII NOCONSOLE
  ? "after TO FILE ASCII: _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-heading.txt") ASCII NOCONSOLE HEADING "The Heading Text"
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-range.txt") ASCII NOCONSOLE RANGE 2, 3
  ? "after RANGE 2,3: _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-summary.txt") ASCII NOCONSOLE SUMMARY
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-plain.txt") ASCII NOCONSOLE PLAIN
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-noeject.txt") ASCII NOCONSOLE NOEJECT
  * ADDITIVE belongs to TO FILE: after RANGE it is error 36 (measured)
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-additive.txt") ASCII NOCONSOLE RANGE 1, 1
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-additive.txt") ADDITIVE ASCII NOCONSOLE RANGE 1, 1
  * a scope or FOR after the other clauses is accepted and matched nothing (measured); the
  * syntax puts them right after the file name, so both orders are tried
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-scope.txt") ASCII NOCONSOLE NEXT 3
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-for.txt") ASCII NOCONSOLE FOR employee.emp_id > "8"
  REPORT FORM (cFrx) NEXT 3 TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-scope2.txt") ASCII NOCONSOLE
  REPORT FORM (cFrx) FOR employee.emp_id > "8" TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-for2.txt") ASCII NOCONSOLE
  IF nBehaviour = 80
    * the bytes of the text form: line ends and page ends
    cRaw = FILETOSTR(gcRun + "fdv-80-ascii.txt")
    ? "80 ASCII text: bytes", LEN(cRaw), "CR", OCCURS(CHR(13), cRaw), "LF", OCCURS(CHR(10), cRaw), "FF", OCCURS(CHR(12), cRaw), "lines", ALINES(aRaw, cRaw), "widest", fdvWidest(cRaw)
  ENDIF
  * NAME: what object it leaves behind
  RELEASE oDE
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-name.txt") ASCII NOCONSOLE NAME oDE
  ? "after NAME oDE: TYPE", TYPE("oDE"), IIF(TYPE("oDE") = "O", oDE.Class + " " + oDE.BaseClass, "")
  IF TYPE("oDE") = "O"
    ? "  oDE.Name", oDE.Name, "cursors", IIF(PEMSTATUS(oDE, "Cursor1", 5), oDE.Cursor1.Alias, "none")
  ENDIF
  RELEASE oDE

  * TO FILE without ASCII: what bytes it writes
  cRaw = gcRun + "raw" + TRANSFORM(nBehaviour) + ".bin"
  ERASE (cRaw)
  REPORT FORM (cFrx) TO FILE (cRaw) NOCONSOLE RANGE 1, 1
  IF FILE(cRaw)
    cHex = ""
    FOR i = 1 TO MIN(200, FSIZE(cRaw))
      cHex = cHex + RIGHT(TRANSFORM(ASC(SUBSTR(FILETOSTR(cRaw), i, 1)), "@0"), 2) + " "
    ENDFOR
    ? "TO FILE without ASCII: LEN", FSIZE(cRaw)
    ? "  first 200 bytes hex:", cHex
    ? "  printable:", CHRTRAN(LEFT(FILETOSTR(cRaw), 300), CHR(0) + CHR(13) + CHR(10) + CHR(12) + CHR(27), ".<>^!")
  ELSE
    ? "TO FILE without ASCII: no file written"
  ENDIF

  * the bare form under 80: what reaches the console (this capture); under 90 it prints, so
  * that one is with the printing statements at the end
  IF nBehaviour = 80
    ? ">>> BEGIN bare REPORT FORM", nBehaviour, "(SET CONSOLE OFF, as the harness has it)"
    _PAGENO = 1
    REPORT FORM (cFrx)
    ? ">>> END bare REPORT FORM; _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
    ? ">>> BEGIN bare REPORT FORM", nBehaviour, "(SET CONSOLE ON)"
    SET CONSOLE ON
    REPORT FORM (cFrx)
    SET CONSOLE OFF
    ? ">>> END bare REPORT FORM with CONSOLE ON; _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
  ENDIF

  * OBJECT ox: what reaches the console beside the listener
  ox = CREATEOBJECT("fdvCounter")
  ox.ListenerType = 3
  ox.QuietMode = .T.
  ? ">>> BEGIN OBJECT ox", nBehaviour
  REPORT FORM (cFrx) OBJECT ox HEADING "Heading Under Object"
  ? ">>> END OBJECT ox; renders", ox.nRenders, "heading seen in Render", ox.lHeading, "pages", ox.OutputPageCount, "_PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
  ox = .NULL.

  * TO FILE ASCII without NOCONSOLE: does the text also reach the console
  ? ">>> BEGIN TO FILE ASCII without NOCONSOLE", nBehaviour
  REPORT FORM (cFrx) TO FILE (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-console.txt") ASCII RANGE 1, 1
  ? ">>> END TO FILE ASCII without NOCONSOLE"

  * the console channel measured on its own: the wrapper's capture is closed and an alternate
  * file of this probe's own is opened in its place, so whatever a bare REPORT FORM sends to the
  * console lands in a file nothing else writes to. SET CONSOLE ON as well, because the wrapper
  * runs with it off.
  SET ALTERNATE OFF
  SET ALTERNATE TO
  SET ALTERNATE TO (gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-alternate.txt")
  SET ALTERNATE ON
  SET CONSOLE ON
  ? "one line from ? before the report"
  REPORT FORM (cFrx) RANGE 1, 1
  ? "one line from ? after the report"
  SET CONSOLE OFF
  SET ALTERNATE OFF
  SET ALTERNATE TO
  SET ALTERNATE TO "out.txt" ADDITIVE
  SET ALTERNATE ON
  ? "own alternate file under", nBehaviour, "bytes", FSIZE(gcRun + "fdv-" + TRANSFORM(nBehaviour) + "-alternate.txt")
ENDFOR

* the preview forms: a window the harness closes with ESC
FOR nBehaviour = 80 TO 90 STEP 10
  SET REPORTBEHAVIOR (nBehaviour)
  gcContext = "preview " + TRANSFORM(nBehaviour)
  ? ">>> BEGIN PREVIEW NOCONSOLE", nBehaviour
  REPORT FORM (cFrx) PREVIEW NOCONSOLE
  ? ">>> END PREVIEW NOCONSOLE; _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
ENDFOR

* the printing statements, last: one page each, and each announced in fdv-err.txt first so a
* run that stops in a dialog says where
SET REPORTBEHAVIOR 80
gcContext = "to printer 80"
STRTOFILE("about to run TO PRINTER NOCONSOLE under 80" + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
? ">>> BEGIN TO PRINTER NOCONSOLE 80"
REPORT FORM (cFrx) TO PRINTER NOCONSOLE RANGE 1, 1
? ">>> END TO PRINTER NOCONSOLE 80; _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
STRTOFILE("TO PRINTER under 80 returned" + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
SET REPORTBEHAVIOR 90
gcContext = "bare 90"
STRTOFILE("about to run bare REPORT FORM under 90" + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
? ">>> BEGIN bare REPORT FORM 90"
_PAGENO = 1
REPORT FORM (cFrx) RANGE 1, 1
? ">>> END bare REPORT FORM 90; _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
STRTOFILE("bare REPORT FORM under 90 returned" + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
gcContext = "to printer 90"
STRTOFILE("about to run TO PRINTER NOCONSOLE under 90" + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
? ">>> BEGIN TO PRINTER NOCONSOLE 90"
REPORT FORM (cFrx) TO PRINTER NOCONSOLE RANGE 1, 1
? ">>> END TO PRINTER NOCONSOLE 90; _PAGENO", _PAGENO, "_PAGETOTAL", _PAGETOTAL
STRTOFILE("TO PRINTER under 90 returned" + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
gcContext = "end"
? "done"
RETURN

FUNCTION fdvWidest
LPARAMETERS cText
LOCAL n, i, nWidest
LOCAL ARRAY aL[1]
n = ALINES(aL, cText)
nWidest = 0
FOR i = 1 TO n
  nWidest = MAX(nWidest, LEN(RTRIM(aL[i])))
ENDFOR
RETURN nWidest
ENDFUNC

PROCEDURE fdvErr
LPARAMETERS nCode, cMsg, nLine, cProg
gnErr = nCode
? "ERROR", nCode, "[" + cMsg + "] line", nLine, "during", gcContext
STRTOFILE("ERROR " + TRANSFORM(nCode) + " [" + cMsg + "] line " + TRANSFORM(nLine) + " in " + cProg + " during " + gcContext + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
RETURN

* Visual FoxPro 9 on this machine cannot open the default printer (SET("PRINTER", 2) raises 125
* "Printer is not ready", the laser 1957 "Error accessing printer spooler"), and an
* object-assisted run needs a printer driver for its page metrics (1958 "Error loading printer
* driver" without one). So the run names a printer first: "Microsoft Print to PDF" when it is
* there, because a job sent to it ends in a Save dialog that dies with the process, where
* another virtual driver may open its own application; otherwise the first one accepted.
PROCEDURE fdvPrinter
LOCAL i, n
LOCAL ARRAY aPr[1]
n = APRINTERS(aPr)
FOR i = 1 TO n
  IF UPPER(aPr[i, 1]) == "MICROSOFT PRINT TO PDF"
    gnErr = 0
    SET PRINTER TO NAME (aPr[i, 1])
    IF gnErr = 0
      ? "printer named for the run:", aPr[i, 1], "on", aPr[i, 2]
      RETURN
    ENDIF
  ENDIF
ENDFOR
FOR i = 1 TO n
  gnErr = 0
  SET PRINTER TO NAME (aPr[i, 1])
  IF gnErr = 0
    ? "printer named for the run:", aPr[i, 1], "on", aPr[i, 2]
    RETURN
  ENDIF
ENDFOR
? "no printer could be named; SET PRINTER TO NAME raised an error for every one of", n
ENDPROC

DEFINE CLASS fdvCounter AS ReportListener
  nRenders = 0
  lHeading = .F.
  PROCEDURE Render
    LPARAMETERS nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage
    THIS.nRenders = THIS.nRenders + 1
    IF "Heading Under Object" $ cContentsToBeRendered
      THIS.lHeading = .T.
    ENDIF
    DODEFAULT(nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage)
  ENDPROC
ENDDEFINE
