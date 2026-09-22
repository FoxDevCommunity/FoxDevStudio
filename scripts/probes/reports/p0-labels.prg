* p0-labels: where a label file keeps its columns and size, and how LABEL FORM fills a page.
*
* A probe, not a golden: run it with
*   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-labels.prg
* cust.lbx (Samples\Solution\Reports) is run through the Render-logging listener, then copies
* of it with the header record's VPOS, HPOS, HEIGHT and WIDTH changed one at a time, so the
* Render log says which of them is the column count, the column width, and the gaps. The copies
* live two levels below a copy of Samples\Data, where the label's data environment looks.
ON ERROR DO fdvErr WITH ERROR(), MESSAGE(), LINENO(), PROGRAM()
SET SAFETY OFF
SET TALK OFF
PUBLIC gcRun, gcContext, gnErr
gcRun = ADDBS(SYS(5) + SYS(2003))
gcContext = "start"
gnErr = 0
LOCAL cLbx, cVar, i, n, ox
LOCAL ARRAY aFiles[1]
cLbx = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\cust.lbx"
cVar = gcRun + "sol\rep\"
fdvPrinter()

MKDIR (gcRun + "data")
MKDIR (gcRun + "sol")
MKDIR (gcRun + "sol\rep")
n = ADIR(aFiles, "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Data\*.*")
FOR i = 1 TO n
  IF INLIST(UPPER(JUSTEXT(aFiles[i, 1])), "DBF", "FPT", "CDX", "DBC", "DCT", "DCX")
    * COPY FILE refuses a database container with error 1102 (measured), so the three
    * database files are copied byte for byte instead
    IF INLIST(UPPER(JUSTEXT(aFiles[i, 1])), "DBC", "DCT", "DCX")
      STRTOFILE(FILETOSTR("C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Data\" + aFiles[i, 1]), gcRun + "data\" + aFiles[i, 1])
    ELSE
      COPY FILE ("C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Data\" + aFiles[i, 1]) TO (gcRun + "data\" + aFiles[i, 1])
    ENDIF
  ENDIF
ENDFOR
COPY FILE ("C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\logo.bmp") TO (cVar + "logo.bmp")

* the file's own header and band numbers
USE (cLbx) AGAIN SHARED ALIAS plbx
SCAN FOR INLIST(objtype, 1, 9, 17, 8)
  fdvLog("LBX R" + TRANSFORM(RECNO()) + " T" + TRANSFORM(objtype) + " C" + TRANSFORM(objcode) + " VPOS " + TRANSFORM(vpos) + " HPOS " + TRANSFORM(hpos) ;
    + " H " + TRANSFORM(height) + " W " + TRANSFORM(width) + " [" + LEFT(CHRTRAN(expr, CHR(13) + CHR(10), "|"), 30) + "] PIC [" + LEFT(picture, 20) + "] plain " + TRANSFORM(plain))
ENDSCAN
COPY TO (cVar + "l_orig.lbx")
COPY TO (cVar + "l_cols3.lbx")
COPY TO (cVar + "l_cols1.lbx")
COPY TO (cVar + "l_hpos.lbx")
COPY TO (cVar + "l_height.lbx")
COPY TO (cVar + "l_width.lbx")
COPY TO (cVar + "l_asfrx.frx")
USE
fdvLog("COPY TO x.lbx makes: lbx " + TRANSFORM(FILE(cVar + "l_orig.lbx")) + " lbt " + TRANSFORM(FILE(cVar + "l_orig.lbt")) + " fpt " + TRANSFORM(FILE(cVar + "l_orig.fpt")))

* the original through the listener: fill order across the page
gcContext = "original"
CD (cVar)
fdvLabel(cLbx, "ORIG")
fdvVariant("l_cols3", "header VPOS 3", "vpos WITH 3")
fdvVariant("l_cols1", "header VPOS 1", "vpos WITH 1")
fdvVariant("l_hpos", "header HPOS 5000", "hpos WITH 5000")
fdvVariant("l_height", "header HEIGHT 5000", "height WITH 5000")
fdvVariant("l_width", "header WIDTH 25000", "width WITH 25000")

* the same file with a report extension, and a report run as a label
gcContext = "cross extension"
fdvLog("--- REPORT FORM over the .lbx copy renamed .frx")
ox = CREATEOBJECT("fdvListener")
ox.cTag = "ASFRX"
ox.ListenerType = 3
ox.QuietMode = .T.
REPORT FORM (cVar + "l_asfrx.frx") OBJECT ox NOCONSOLE
fdvLog("ASFRX pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders))
ox = .NULL.
fdvLog("--- LABEL FORM over percent.frx")
ox = CREATEOBJECT("fdvListener")
ox.cTag = "FRXASLBL"
ox.ListenerType = 3
ox.QuietMode = .T.
LABEL FORM "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\percent.frx" OBJECT ox NOCONSOLE
fdvLog("FRXASLBL pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders))
ox = .NULL.

* the text form under 80, and 90
gcContext = "ascii"
SET REPORTBEHAVIOR 80
LABEL FORM (cLbx) TO FILE (gcRun + "fdv-80-ascii.txt") ASCII NOCONSOLE
SET REPORTBEHAVIOR 90
LABEL FORM (cLbx) TO FILE (gcRun + "fdv-90-ascii.txt") ASCII NOCONSOLE
SET REPORTBEHAVIOR 80

* SAMPLE, last: it prints one sample label and asks whether to print another
gcContext = "sample"
fdvLog("about to run LABEL FORM ... SAMPLE TO FILE ASCII")
LABEL FORM (cLbx) SAMPLE TO FILE (gcRun + "fdv-sample.txt") ASCII NOCONSOLE
fdvLog("SAMPLE returned; file " + TRANSFORM(FILE(gcRun + "fdv-sample.txt")))
CD (gcRun)
gcContext = "end"
fdvLog("done")
RETURN

PROCEDURE fdvVariant
LPARAMETERS cName, cWhat, cReplace
gcContext = cName
USE (gcRun + "sol\rep\" + cName + ".lbx") EXCLUSIVE
GO TOP
LOCATE FOR objtype = 1
REPLACE &cReplace
USE
fdvLabel(gcRun + "sol\rep\" + cName + ".lbx", cName + " (" + cWhat + ")")
ENDPROC

PROCEDURE fdvLabel
LPARAMETERS cFile, cTag
LOCAL ox
fdvLog("--- " + cTag)
ox = CREATEOBJECT("fdvListener")
ox.cTag = cTag
ox.ListenerType = 3
ox.QuietMode = .T.
LABEL FORM (cFile) OBJECT ox NOCONSOLE
fdvLog(cTag + " pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders) + " page " + TRANSFORM(ox.nPageW) + "x" + TRANSFORM(ox.nPageH))
ox = .NULL.
ENDPROC

PROCEDURE fdvLog
LPARAMETERS cText
STRTOFILE(cText + CHR(13) + CHR(10), gcRun + "fdv-log.txt", .T.)
ENDPROC

PROCEDURE fdvErr
LPARAMETERS nCode, cMsg, nLine, cProg
gnErr = nCode
STRTOFILE("ERROR " + TRANSFORM(nCode) + " [" + cMsg + "] line " + TRANSFORM(nLine) + " in " + cProg + " during " + gcContext + CHR(13) + CHR(10), gcRun + "fdv-log.txt", .T.)
RETURN

* Visual FoxPro 9 on this machine cannot open the default printer (SET("PRINTER", 2) raises 125
* "Printer is not ready", the laser 1957 "Error accessing printer spooler"), and an
* object-assisted run needs a printer driver for its page metrics (1958 "Error loading printer
* driver" without one). So the run names a printer first: "Microsoft Print to PDF" when it is
* there, otherwise the first one accepted. Nothing here prints to it.
PROCEDURE fdvPrinter
LOCAL i, n
LOCAL ARRAY aPr[1]
n = APRINTERS(aPr)
FOR i = 1 TO n
  IF UPPER(aPr[i, 1]) == "MICROSOFT PRINT TO PDF"
    gnErr = 0
    SET PRINTER TO NAME (aPr[i, 1])
    IF gnErr = 0
      fdvLog("printer named for the run: " + aPr[i, 1] + " on " + aPr[i, 2])
      RETURN
    ENDIF
  ENDIF
ENDFOR
FOR i = 1 TO n
  gnErr = 0
  SET PRINTER TO NAME (aPr[i, 1])
  IF gnErr = 0
    fdvLog("printer named for the run: " + aPr[i, 1] + " on " + aPr[i, 2])
    RETURN
  ENDIF
ENDFOR
fdvLog("no printer could be named; SET PRINTER TO NAME raised an error for every one of " + TRANSFORM(n))
ENDPROC

DEFINE CLASS fdvListener AS ReportListener
  cTag = ""
  nRenders = 0
  nPageW = 0
  nPageH = 0

  PROCEDURE BeforeReport
    THIS.nPageW = THIS.GetPageWidth()
    THIS.nPageH = THIS.GetPageHeight()
    DODEFAULT()
  ENDPROC

  PROCEDURE Render
    LPARAMETERS nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage
    THIS.nRenders = THIS.nRenders + 1
    IF THIS.nRenders <= 200
      STRTOFILE(THIS.cTag + " P" + TRANSFORM(THIS.PageNo) + " R" + TRANSFORM(nFRXRecNo) + " " + TRANSFORM(nLeft) + " " + TRANSFORM(nTop) ;
        + " " + TRANSFORM(nWidth) + " " + TRANSFORM(nHeight) + " c" + TRANSFORM(nObjectContinuationType) ;
        + " [" + LEFT(CHRTRAN(cContentsToBeRendered, CHR(13) + CHR(10), "|"), 120) + "]" + CHR(13) + CHR(10), gcRun + "fdv-render.txt", .T.)
    ENDIF
    DODEFAULT(nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage)
  ENDPROC
ENDDEFINE
