* p0-printers: what the printer functions answer on a machine with printers.
*
* A probe, not a golden: run it with
*   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-printers.prg
* Everything that answers without asking is measured first. GETPRINTER() and SYS(1037) open
* Windows dialogs that Visual FoxPro's own timer cannot see past, so they come last, each
* announced in fdv-log.txt before it is called: a run that is killed says which one hung.
ON ERROR DO fdvErr WITH ERROR(), MESSAGE(), LINENO(), PROGRAM()
SET SAFETY OFF
SET TALK OFF
PUBLIC gcRun, gcContext, gnErr
gcRun = ADDBS(SYS(5) + SYS(2003))
gcContext = "start"
gnErr = 0
LOCAL n, i, k, cLine, cPrinter, cAnswer, ox
LOCAL ARRAY aP[1], aQ[1]

gcContext = "APRINTERS"
n = APRINTERS(aP)
fdvLog("APRINTERS(a) = " + TRANSFORM(n) + " columns " + TRANSFORM(ALEN(aP, 2)))
FOR i = 1 TO n
  cLine = "  " + TRANSFORM(i)
  FOR k = 1 TO ALEN(aP, 2)
    cLine = cLine + " [" + TRANSFORM(aP[i, k]) + "]"
  ENDFOR
  fdvLog(cLine)
ENDFOR
n = APRINTERS(aQ, 1)
fdvLog("APRINTERS(a, 1) = " + TRANSFORM(n) + " columns " + TRANSFORM(ALEN(aQ, 2)))
FOR i = 1 TO n
  cLine = "  " + TRANSFORM(i)
  FOR k = 1 TO ALEN(aQ, 2)
    cLine = cLine + " [" + TRANSFORM(aQ[i, k]) + "]"
  ENDFOR
  fdvLog(cLine)
ENDFOR

gcContext = "SET PRINTER"
fdvLog("SET(PRINTER) [" + TRANSFORM(SET("PRINTER")) + "] SET(PRINTER, 1) [" + TRANSFORM(SET("PRINTER", 1)) + "] SET(PRINTER, 2) [" + TRANSFORM(SET("PRINTER", 2)) + "] SET(PRINTER, 3) [" + TRANSFORM(SET("PRINTER", 3)) + "]")
cPrinter = SET("PRINTER", 2)

gcContext = "PRINTSTATUS"
fdvLog("PRINTSTATUS() = " + TRANSFORM(PRINTSTATUS()))

gcContext = "PRTINFO"
FOR i = 1 TO 13
  fdvLog("PRTINFO(" + TRANSFORM(i) + ") = " + TRANSFORM(PRTINFO(i)) + "   PRTINFO(" + TRANSFORM(i) + ", default) = " + TRANSFORM(PRTINFO(i, cPrinter)))
ENDFOR
fdvLog("PRTINFO(14) = " + TRANSFORM(PRTINFO(14)))
fdvLog("PRTINFO(0) = " + TRANSFORM(PRTINFO(0)))
fdvLog("PRTINFO(1, nosuch) = " + TRANSFORM(PRTINFO(1, "No Such Printer")))
FOR i = 1 TO n
  fdvLog("PRTINFO(2, " + aQ[i, 1] + ") = " + TRANSFORM(PRTINFO(2, aQ[i, 1])) + " paper length " + TRANSFORM(PRTINFO(3, aQ[i, 1])) + " width " + TRANSFORM(PRTINFO(4, aQ[i, 1])) + " colour " + TRANSFORM(PRTINFO(9, aQ[i, 1])))
ENDFOR

* which printers Visual FoxPro can name, and what a listener sees of the page once one is named
gcContext = "SET PRINTER TO NAME"
FOR i = 1 TO n
  gnErr = 0
  SET PRINTER TO NAME (aQ[i, 1])
  IF gnErr = 0
    ox = CREATEOBJECT("ReportListener")
    ox.ListenerType = 3
    ox.QuietMode = .T.
    REPORT FORM "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\percent.frx" OBJECT ox NOCONSOLE
    fdvLog("SET PRINTER TO NAME " + aQ[i, 1] + ": accepted; SET(PRINTER, 2) now [" + fdvSetPrinter2() + "]; PRTINFO(1) " + TRANSFORM(PRTINFO(1)) ;
      + " PRTINFO(2) " + TRANSFORM(PRTINFO(2)) + " PRTINFO(3) " + TRANSFORM(PRTINFO(3)) + " PRTINFO(4) " + TRANSFORM(PRTINFO(4)) ;
      + "; a type 3 run of percent.frx: pages " + TRANSFORM(ox.OutputPageCount) + " GetPageWidth " + TRANSFORM(ox.GetPageWidth()) + " GetPageHeight " + TRANSFORM(ox.GetPageHeight()))
    ox = .NULL.
  ELSE
    fdvLog("SET PRINTER TO NAME " + aQ[i, 1] + ": error " + TRANSFORM(gnErr))
  ENDIF
ENDFOR
SET PRINTER TO
fdvLog("SET PRINTER TO (reset): SET(PRINTER, 2) [" + fdvSetPrinter2() + "]")

gcContext = "SYS(1037) and GETPRINTER"
fdvLog("SYS(2040) = " + SYS(2040))
fdvLog("about to call GETPRINTER()")
cAnswer = GETPRINTER()
fdvLog("GETPRINTER() returned [" + cAnswer + "]")
fdvLog("about to call SYS(1037)")
cAnswer = SYS(1037)
fdvLog("SYS(1037) returned [" + cAnswer + "]")
gcContext = "end"
fdvLog("done")
RETURN

PROCEDURE fdvLog
LPARAMETERS cText
STRTOFILE(cText + CHR(13) + CHR(10), gcRun + "fdv-log.txt", .T.)
ENDPROC

PROCEDURE fdvErr
LPARAMETERS nCode, cMsg, nLine, cProg
gnErr = nCode
STRTOFILE("ERROR " + TRANSFORM(nCode) + " [" + cMsg + "] line " + TRANSFORM(nLine) + " in " + cProg + " during " + gcContext + CHR(13) + CHR(10), gcRun + "fdv-log.txt", .T.)
RETURN

* SET("PRINTER", 2) raises 125 when Visual FoxPro has no printer it can open; this answers the
* error number as text instead of stopping the line that asked.
FUNCTION fdvSetPrinter2
LOCAL cAnswer
gnErr = 0
cAnswer = SET("PRINTER", 2)
IF gnErr <> 0
  RETURN "error " + TRANSFORM(gnErr)
ENDIF
RETURN cAnswer
ENDFUNC
