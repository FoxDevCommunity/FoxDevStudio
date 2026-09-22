* p0-listener-types: which ReportListener types run headless, what Render is handed, in what unit.
*
* A probe, not a golden: run it with
*   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-listener-types.prg
* and read the fdv-*.txt files it prints. Every measurement is written with STRTOFILE the moment
* it is made, so a run that has to be killed still hands back what it measured before the
* statement that hung. The probe ends with RETURN: the wrapper QUITs and records how it ended.
*
* percent.frx is run once per ListenerType: 3, 2 and 1 first, then -1 and 0 last because those
* two print, and on this machine a print job ends in a dialog (see fdvPrinter below), so a run
* that hangs there loses nothing measured before it. A subclass logs every Render call to
* fdv-render.txt and every other event to fdv-events.txt. The file's own HPOS/VPOS/WIDTH/HEIGHT
* for the records the log names are written first, so the unit of Render's coordinates can be
* read off by comparing the two.
ON ERROR DO fdvErr WITH ERROR(), MESSAGE(), LINENO(), PROGRAM()
SET SAFETY OFF
SET TALK OFF
PUBLIC gcRun, gcContext, gnErr
gcRun = ADDBS(SYS(5) + SYS(2003))
gcContext = "start"
gnErr = 0
LOCAL cFrx, cRep, ox, nType, i, j, nMembers, cLine
LOCAL ARRAY aList[5], aCC[1, 2]
cRep = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\"
cFrx = cRep + "percent.frx"
fdvLog("fdv-log.txt", "REPORTBEHAVIOR at start: " + TRANSFORM(SET("REPORTBEHAVIOR")) + " type " + VARTYPE(SET("REPORTBEHAVIOR")))
fdvPrinter()

* the file's own numbers, for the unit check against the Render log
USE (cFrx) AGAIN SHARED ALIAS pfrx
SCAN FOR INLIST(objtype, 1, 5, 6, 7, 8, 9, 17)
  fdvLog("fdv-frx.txt", "R" + TRANSFORM(RECNO()) + " T" + TRANSFORM(objtype) + " C" + TRANSFORM(objcode) ;
    + " HPOS " + TRANSFORM(hpos) + " VPOS " + TRANSFORM(vpos) + " W " + TRANSFORM(width) + " H " + TRANSFORM(height) ;
    + " [" + LEFT(CHRTRAN(expr, CHR(13) + CHR(10), "|"), 40) + "]")
ENDSCAN
USE

aList[1] = 3
aList[2] = 2
aList[3] = 1
aList[4] = -1
aList[5] = 0
FOR i = 1 TO 3
  nType = aList[i]
  fdvType(nType)
ENDFOR

* CancelReport from inside the run: what it stops
gcContext = "cancel during run"
ox = CREATEOBJECT("fdvListener")
ox.cTag = "CANCEL2"
ox.ListenerType = 3
ox.QuietMode = .T.
ox.nCancelAtRender = 10
CD (cRep)
REPORT FORM (cFrx) OBJECT ox NOCONSOLE
CD (gcRun)
fdvLog("fdv-log.txt", "CANCEL2 after: renders " + TRANSFORM(ox.nRenders) + " renders after cancel " + TRANSFORM(ox.nAfterCancel) ;
  + " PageTotal " + TRANSFORM(ox.PageTotal) + " OutputPageCount " + TRANSFORM(ox.OutputPageCount) + " PageNo " + TRANSFORM(ox.PageNo) + " _PAGENO " + TRANSFORM(_PAGENO))
ox = .NULL.

* TwoPassProcess: what the second pass looks like
gcContext = "two pass"
ox = CREATEOBJECT("fdvListener")
ox.cTag = "TWOPASS"
ox.ListenerType = 3
ox.QuietMode = .T.
ox.TwoPassProcess = .T.
CD (cRep)
REPORT FORM (cFrx) OBJECT ox NOCONSOLE
CD (gcRun)
fdvLog("fdv-log.txt", "TWOPASS after: renders " + TRANSFORM(ox.nRenders) + " PageTotal " + TRANSFORM(ox.PageTotal) + " OutputPageCount " + TRANSFORM(ox.OutputPageCount) ;
  + " CurrentPass " + TRANSFORM(ox.CurrentPass) + " _PAGETOTAL " + TRANSFORM(_PAGETOTAL))
ox = .NULL.

* the base class, no override at all: does it still produce pages headless
gcContext = "base class"
ox = CREATEOBJECT("ReportListener")
ox.ListenerType = 3
ox.QuietMode = .T.
CD (cRep)
REPORT FORM (cFrx) OBJECT ox NOCONSOLE
CD (gcRun)
fdvLog("fdv-log.txt", "BASE after: PageTotal " + TRANSFORM(ox.PageTotal) + " OutputPageCount " + TRANSFORM(ox.OutputPageCount) + " PageNo " + TRANSFORM(ox.PageNo) ;
  + " _PAGENO " + TRANSFORM(_PAGENO) + " _PAGETOTAL " + TRANSFORM(_PAGETOTAL) + " Class " + ox.Class + " BaseClass " + ox.BaseClass)
ox = .NULL.

* a listener whose events are logged but which does not override Render: are the events the same
gcContext = "no render override"
ox = CREATEOBJECT("fdvEventsOnly")
ox.cTag = "EVONLY"
ox.ListenerType = 3
ox.QuietMode = .T.
CD (cRep)
REPORT FORM (cFrx) OBJECT ox NOCONSOLE
CD (gcRun)
fdvLog("fdv-log.txt", "EVONLY after: PageTotal " + TRANSFORM(ox.PageTotal) + " OutputPageCount " + TRANSFORM(ox.OutputPageCount))
ox = .NULL.

* the system variables that pick a listener when OBJECT TYPE is used
fdvLog("fdv-log.txt", "_REPORTOUTPUT " + TRANSFORM(_REPORTOUTPUT) + " _REPORTPREVIEW " + TRANSFORM(_REPORTPREVIEW) + " _REPORTBUILDER " + TRANSFORM(_REPORTBUILDER))

* OBJECT TYPE 1: it opens a preview, which the harness closes with ESC
gcContext = "object type 1"
fdvLog("fdv-log.txt", "about to run OBJECT TYPE 1")
CD (cRep)
REPORT FORM (cFrx) OBJECT TYPE 1 NOCONSOLE
CD (gcRun)
fdvLog("fdv-log.txt", "OBJECT TYPE 1 returned; _PAGENO " + TRANSFORM(_PAGENO) + " _PAGETOTAL " + TRANSFORM(_PAGETOTAL))

* the printing types last
FOR i = 4 TO 5
  fdvLog("fdv-log.txt", "about to run ListenerType " + TRANSFORM(aList[i]))
  fdvType(aList[i])
ENDFOR
gcContext = "end"
fdvLog("fdv-log.txt", "done")
RETURN

* One run of percent.frx through the logging listener at the given ListenerType.
PROCEDURE fdvType
LPARAMETERS nType
LOCAL ox, j, nMembers
LOCAL ARRAY aCC[1, 2]
  cRep = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\"
  cFrx = cRep + "percent.frx"
  gcContext = "type " + TRANSFORM(nType)
  ox = CREATEOBJECT("fdvListener")
  ox.cTag = "T" + TRANSFORM(nType)
  ox.ListenerType = nType
  ox.QuietMode = .T.
  fdvLog("fdv-log.txt", ox.cTag + " before: ListenerType " + TRANSFORM(ox.ListenerType) + " CurrentPass " + TRANSFORM(ox.CurrentPass) ;
    + " TwoPassProcess " + TRANSFORM(ox.TwoPassProcess) + " PageNo " + TRANSFORM(ox.PageNo) + " PageTotal " + TRANSFORM(ox.PageTotal) ;
    + " OutputPageCount " + TRANSFORM(ox.OutputPageCount) + " GetPageWidth " + TRANSFORM(ox.GetPageWidth()) + " GetPageHeight " + TRANSFORM(ox.GetPageHeight()) ;
    + " Supports(" + TRANSFORM(nType) + ") " + TRANSFORM(ox.SupportsListenerType(nType)))
  _PAGENO = 1
  CD (cRep)
  IF nType <= 0
    * printer output: one page on paper is enough to see what the listener is handed
    REPORT FORM (cFrx) OBJECT ox NOCONSOLE RANGE 1, 1
  ELSE
    REPORT FORM (cFrx) OBJECT ox NOCONSOLE
  ENDIF
  CD (gcRun)
  fdvLog("fdv-log.txt", ox.cTag + " after: ListenerType " + TRANSFORM(ox.ListenerType) + " CurrentPass " + TRANSFORM(ox.CurrentPass) ;
    + " PageNo " + TRANSFORM(ox.PageNo) + " PageTotal " + TRANSFORM(ox.PageTotal) + " OutputPageCount " + TRANSFORM(ox.OutputPageCount) ;
    + " GetPageWidth " + TRANSFORM(ox.GetPageWidth()) + " GetPageHeight " + TRANSFORM(ox.GetPageHeight()) ;
    + " renders " + TRANSFORM(ox.nRenders) + " _PAGENO " + TRANSFORM(_PAGENO) + " _PAGETOTAL " + TRANSFORM(_PAGETOTAL) ;
    + " SET(REPORTBEHAVIOR) " + TRANSFORM(SET("REPORTBEHAVIOR")))
  IF nType = 3
    nMembers = AMEMBERS(aCC, ox.CommandClauses, 1)
    FOR j = 1 TO nMembers
      IF aCC[j, 2] = "Property"
        fdvLog("fdv-log.txt", "  CommandClauses." + aCC[j, 1] + " = " + TRANSFORM(EVALUATE("ox.CommandClauses." + aCC[j, 1])))
      ENDIF
    ENDFOR
  ENDIF
  ox.CancelReport()
  fdvLog("fdv-log.txt", ox.cTag + " after CancelReport: PageTotal " + TRANSFORM(ox.PageTotal) + " OutputPageCount " + TRANSFORM(ox.OutputPageCount) + " PageNo " + TRANSFORM(ox.PageNo))
  ox = .NULL.
ENDPROC

* Visual FoxPro 9 on this machine cannot open the default printer (SET("PRINTER", 2) raises 125
* "Printer is not ready", the laser 1957 "Error accessing printer spooler"), and an
* object-assisted run needs a printer driver for its page metrics (1958 "Error loading printer
* driver" without one). So the run names a printer first: "Microsoft Print to PDF" when it is
* there, because a job sent to it ends in a Save dialog that dies with the process, where
* another virtual driver may open its own application; otherwise the first one accepted.
PROCEDURE fdvPrinter
LOCAL i, n, cName
LOCAL ARRAY aPr[1]
n = APRINTERS(aPr)
FOR i = 1 TO n
  IF UPPER(aPr[i, 1]) == "MICROSOFT PRINT TO PDF"
    gnErr = 0
    SET PRINTER TO NAME (aPr[i, 1])
    IF gnErr = 0
      fdvLog("fdv-log.txt", "printer named for the run: " + aPr[i, 1] + " on " + aPr[i, 2])
      RETURN
    ENDIF
  ENDIF
ENDFOR
FOR i = 1 TO n
  gnErr = 0
  SET PRINTER TO NAME (aPr[i, 1])
  IF gnErr = 0
    fdvLog("fdv-log.txt", "printer named for the run: " + aPr[i, 1] + " on " + aPr[i, 2])
    RETURN
  ENDIF
ENDFOR
fdvLog("fdv-log.txt", "no printer could be named; SET PRINTER TO NAME raised an error for every one of " + TRANSFORM(n))
ENDPROC

PROCEDURE fdvLog
LPARAMETERS cFile, cText
STRTOFILE(cText + CHR(13) + CHR(10), cFile, .T.)
ENDPROC

* A value as text, whatever its type: an object by its class, everything else through TRANSFORM.
FUNCTION fdvShow
LPARAMETERS xValue
IF VARTYPE(xValue) = "O"
  RETURN "(object " + xValue.Class + ")"
ENDIF
RETURN TRANSFORM(xValue)
ENDFUNC

PROCEDURE fdvErr
LPARAMETERS nCode, cMsg, nLine, cProg
gnErr = nCode
STRTOFILE("ERROR " + TRANSFORM(nCode) + " [" + cMsg + "] line " + TRANSFORM(nLine) + " in " + cProg + " during " + gcContext + CHR(13) + CHR(10), gcRun + "fdv-err.txt", .T.)
RETURN

* The listener under measurement: every call written down with its arguments.
DEFINE CLASS fdvListener AS ReportListener
  cTag = ""
  nRenders = 0
  nAfterCancel = 0
  nCancelAtRender = 0
  lCancelled = .F.
  lMembers = .F.
  lLogEvents = .T.

  PROCEDURE Ev
    LPARAMETERS cText
    IF THIS.lLogEvents
      STRTOFILE(THIS.cTag + " " + cText + " [pass " + TRANSFORM(THIS.CurrentPass) + " page " + TRANSFORM(THIS.PageNo) ;
        + "/" + TRANSFORM(THIS.PageTotal) + " out " + TRANSFORM(THIS.OutputPageCount) + " _PAGENO " + TRANSFORM(_PAGENO) + "]" + CHR(13) + CHR(10), gcRun + "fdv-events.txt", .T.)
    ENDIF
  ENDPROC

  PROCEDURE Render
    LPARAMETERS nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage
    THIS.nRenders = THIS.nRenders + 1
    IF THIS.lCancelled
      THIS.nAfterCancel = THIS.nAfterCancel + 1
    ENDIF
    STRTOFILE(THIS.cTag + " P" + TRANSFORM(THIS.PageNo) + " R" + TRANSFORM(nFRXRecNo) + " " + TRANSFORM(nLeft) + " " + TRANSFORM(nTop) ;
      + " " + TRANSFORM(nWidth) + " " + TRANSFORM(nHeight) + " c" + TRANSFORM(nObjectContinuationType) + " img" + TRANSFORM(GDIPlusImage) ;
      + " [" + LEFT(CHRTRAN(cContentsToBeRendered, CHR(13) + CHR(10), "|"), 70) + "]" + CHR(13) + CHR(10), gcRun + "fdv-render.txt", .T.)
    IF THIS.nCancelAtRender > 0 AND THIS.nRenders >= THIS.nCancelAtRender AND NOT THIS.lCancelled
      THIS.lCancelled = .T.
      THIS.Ev("CancelReport called from Render number " + TRANSFORM(THIS.nRenders) + " on page " + TRANSFORM(THIS.PageNo))
      THIS.CancelReport()
    ENDIF
    DODEFAULT(nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage)
  ENDPROC

  PROCEDURE LoadReport
    THIS.Ev("LoadReport")
    DODEFAULT()
  ENDPROC

  PROCEDURE UnloadReport
    THIS.Ev("UnloadReport")
    DODEFAULT()
  ENDPROC

  PROCEDURE BeforeReport
    THIS.Ev("BeforeReport GetPageWidth " + TRANSFORM(THIS.GetPageWidth()) + " GetPageHeight " + TRANSFORM(THIS.GetPageHeight()) ;
      + " FRXDataSession " + TRANSFORM(THIS.FRXDataSession) + " CurrentDataSession " + TRANSFORM(THIS.CurrentDataSession) ;
      + " SET(DATASESSION) " + TRANSFORM(SET("DATASESSION")) + " ListenerType " + TRANSFORM(THIS.ListenerType) ;
      + " CommandClauses.RECORDTOTAL " + TRANSFORM(THIS.CommandClauses.RECORDTOTAL) + " PrintJobName [" + TRANSFORM(THIS.PrintJobName) + "]")
    DODEFAULT()
  ENDPROC

  PROCEDURE AfterReport
    THIS.Ev("AfterReport GetPageWidth " + TRANSFORM(THIS.GetPageWidth()) + " GetPageHeight " + TRANSFORM(THIS.GetPageHeight()))
    DODEFAULT()
  ENDPROC

  PROCEDURE BeforeBand
    LPARAMETERS nBandObjCode, nFRXRecNo
    THIS.Ev("BeforeBand code " + TRANSFORM(nBandObjCode) + " R" + TRANSFORM(nFRXRecNo))
    DODEFAULT(nBandObjCode, nFRXRecNo)
  ENDPROC

  PROCEDURE AfterBand
    LPARAMETERS nBandObjCode, nFRXRecNo
    THIS.Ev("AfterBand code " + TRANSFORM(nBandObjCode) + " R" + TRANSFORM(nFRXRecNo))
    DODEFAULT(nBandObjCode, nFRXRecNo)
  ENDPROC

  PROCEDURE EvaluateContents
    LPARAMETERS nFRXRecNo, oObjProperties
    LOCAL n, k, cLine
    LOCAL ARRAY aM[1, 2]
    IF NOT THIS.lMembers
      THIS.lMembers = .T.
      n = AMEMBERS(aM, oObjProperties, 1)
      cLine = "EvaluateContents object: VARTYPE " + VARTYPE(oObjProperties) + " members"
      FOR k = 1 TO n
        IF aM[k, 2] = "Property"
          cLine = cLine + " " + aM[k, 1] + "=" + fdvShow(EVALUATE("oObjProperties." + aM[k, 1]))
        ELSE
          cLine = cLine + " " + aM[k, 1] + "(" + aM[k, 2] + ")"
        ENDIF
      ENDFOR
      THIS.Ev(cLine)
    ENDIF
    THIS.Ev("EvaluateContents R" + TRANSFORM(nFRXRecNo) + " Text [" + LEFT(CHRTRAN(TRANSFORM(oObjProperties.Text), CHR(13) + CHR(10), "|"), 40) + "] Reload " + TRANSFORM(oObjProperties.Reload))
    DODEFAULT(nFRXRecNo, oObjProperties)
  ENDPROC

  PROCEDURE AdjustObjectSize
    LPARAMETERS nFRXRecNo, oObjProperties
    LOCAL n, k, cLine
    LOCAL ARRAY aM[1, 2]
    n = AMEMBERS(aM, oObjProperties, 1)
    cLine = "AdjustObjectSize R" + TRANSFORM(nFRXRecNo) + " members"
    FOR k = 1 TO n
      IF aM[k, 2] = "Property"
        cLine = cLine + " " + aM[k, 1] + "=" + fdvShow(EVALUATE("oObjProperties." + aM[k, 1]))
      ENDIF
    ENDFOR
    THIS.Ev(cLine)
    DODEFAULT(nFRXRecNo, oObjProperties)
  ENDPROC

  PROCEDURE OutputPage
    LPARAMETERS nPageNo, eDevice, nDeviceType, nLeft, nTop, nWidth, nHeight, nClipLeft, nClipTop, nClipWidth, nClipHeight
    THIS.Ev("OutputPage PCOUNT " + TRANSFORM(PCOUNT()) + " page " + TRANSFORM(nPageNo) + " device " + fdvShow(eDevice) + " (" + VARTYPE(eDevice) + ") type " + TRANSFORM(nDeviceType) ;
      + " box " + TRANSFORM(nLeft) + "," + TRANSFORM(nTop) + " " + TRANSFORM(nWidth) + "x" + TRANSFORM(nHeight) ;
      + " clip " + TRANSFORM(nClipLeft) + "," + TRANSFORM(nClipTop) + " " + TRANSFORM(nClipWidth) + "x" + TRANSFORM(nClipHeight))
    IF PCOUNT() <= 3
      DODEFAULT(nPageNo, eDevice, nDeviceType)
    ELSE
      DODEFAULT(nPageNo, eDevice, nDeviceType, nLeft, nTop, nWidth, nHeight, nClipLeft, nClipTop, nClipWidth, nClipHeight)
    ENDIF
  ENDPROC

  PROCEDURE IncludePageInOutput
    LPARAMETERS nPageNo
    LOCAL lAnswer
    lAnswer = DODEFAULT(nPageNo)
    THIS.Ev("IncludePageInOutput page " + TRANSFORM(nPageNo) + " = " + TRANSFORM(lAnswer))
    RETURN lAnswer
  ENDPROC

  PROCEDURE DoStatus
    LPARAMETERS cMessage
    THIS.Ev("DoStatus [" + TRANSFORM(cMessage) + "]")
    DODEFAULT(cMessage)
  ENDPROC

  PROCEDURE ClearStatus
    THIS.Ev("ClearStatus")
    DODEFAULT()
  ENDPROC

  PROCEDURE OnPreviewClose
    LPARAMETERS lPrint
    THIS.Ev("OnPreviewClose print " + TRANSFORM(lPrint))
    DODEFAULT(lPrint)
  ENDPROC
ENDDEFINE

* The same events, no Render override: whether Render being overridden changes what else fires.
DEFINE CLASS fdvEventsOnly AS ReportListener
  cTag = ""

  PROCEDURE Ev
    LPARAMETERS cText
    STRTOFILE(THIS.cTag + " " + cText + " [pass " + TRANSFORM(THIS.CurrentPass) + " page " + TRANSFORM(THIS.PageNo) ;
      + "/" + TRANSFORM(THIS.PageTotal) + " out " + TRANSFORM(THIS.OutputPageCount) + "]" + CHR(13) + CHR(10), gcRun + "fdv-events.txt", .T.)
  ENDPROC

  PROCEDURE BeforeReport
    THIS.Ev("BeforeReport")
    DODEFAULT()
  ENDPROC

  PROCEDURE AfterReport
    THIS.Ev("AfterReport")
    DODEFAULT()
  ENDPROC

  PROCEDURE BeforeBand
    LPARAMETERS nBandObjCode, nFRXRecNo
    THIS.Ev("BeforeBand code " + TRANSFORM(nBandObjCode) + " R" + TRANSFORM(nFRXRecNo))
    DODEFAULT(nBandObjCode, nFRXRecNo)
  ENDPROC

  PROCEDURE OutputPage
    LPARAMETERS nPageNo, eDevice, nDeviceType, nLeft, nTop, nWidth, nHeight, nClipLeft, nClipTop, nClipWidth, nClipHeight
    THIS.Ev("OutputPage page " + TRANSFORM(nPageNo) + " type " + TRANSFORM(nDeviceType))
    DODEFAULT(nPageNo, eDevice, nDeviceType, nLeft, nTop, nWidth, nHeight, nClipLeft, nClipTop, nClipWidth, nClipHeight)
  ENDPROC
ENDDEFINE
