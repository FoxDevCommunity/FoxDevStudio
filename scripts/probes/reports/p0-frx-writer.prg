* p0-frx-writer: what a written .frx must contain for Visual FoxPro to run it.
*
* A probe, not a golden: run it with
*   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-frx-writer.prg
* Variants of percent.frx are made with COPY TO and then DELETE/REPLACE/PACK, each run with
* REPORT FORM ... TO FILE ... ASCII NOCONSOLE, and fdv-log.txt says which ran, which raised
* which error, and whether the text produced is byte-for-byte the original's. The data
* environment of percent.frx names ..\..\data\testdata.dbc, which Visual FoxPro resolves against
* the report's own folder (p0-layout-rules measures that), so the variants live two levels below
* a copy of Samples\Data made inside the run directory.
ON ERROR DO fdvErr WITH ERROR(), MESSAGE(), LINENO(), PROGRAM()
SET SAFETY OFF
SET TALK OFF
PUBLIC gcRun, gcContext, gcPercent, gcVar, gcBaseline
gcRun = ADDBS(SYS(5) + SYS(2003))
gcContext = "start"
gcPercent = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\percent.frx"
gcVar = gcRun + "sol\rep\"
LOCAL i, n, c, h
LOCAL ARRAY aFiles[1]

* the data the report's environment opens, two levels above the variants
MKDIR (gcRun + "data")
MKDIR (gcRun + "sol")
MKDIR (gcRun + "sol\rep")
n = ADIR(aFiles, "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Data\*.*")
FOR i = 1 TO n
  IF INLIST(UPPER(JUSTEXT(aFiles[i, 1])), "DBF", "FPT", "CDX", "DBC", "DCT", "DCX")
    * COPY FILE refuses a database container with error 1102 (measured), so the three database
    * files are copied byte for byte instead
    IF INLIST(UPPER(JUSTEXT(aFiles[i, 1])), "DBC", "DCT", "DCX")
      STRTOFILE(FILETOSTR("C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Data\" + aFiles[i, 1]), gcRun + "data\" + aFiles[i, 1])
    ELSE
      COPY FILE ("C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Data\" + aFiles[i, 1]) TO (gcRun + "data\" + aFiles[i, 1])
    ENDIF
  ENDIF
ENDFOR

* the original, from its own folder, is the baseline every variant is compared with
gcContext = "baseline"
REPORT FORM (gcPercent) TO FILE (gcRun + "fdv-v-original.txt") ASCII NOCONSOLE
gcBaseline = FILETOSTR(gcRun + "fdv-v-original.txt")
fdvLog("baseline text " + TRANSFORM(LEN(gcBaseline)) + " bytes")

* SYS(2015) three times, beside the UNIQUEIDs the file carries
fdvLog("SYS(2015): " + SYS(2015) + " " + SYS(2015) + " " + SYS(2015))

* the raw TIMESTAMP values, and what they are if read as a DOS packed date and time
USE (gcPercent) AGAIN SHARED ALIAS pfrx
SCAN FOR timestamp <> 0
  fdvLog("R" + TRANSFORM(RECNO()) + " T" + TRANSFORM(objtype) + " UNIQUEID [" + uniqueid + "] TIMESTAMP " + TRANSFORM(timestamp) + " as DOS date " + fdvDosDate(timestamp))
  IF RECNO() > 12
    EXIT
  ENDIF
ENDSCAN
fdvLog("records " + TRANSFORM(RECCOUNT()) + " codepage " + TRANSFORM(CPDBF()))
USE
fdvLog("memo block size (frt bytes 6-7, big-endian) of the original: " + TRANSFORM(fdvBlockSize(FORCEEXT(gcPercent, "frt"))))

* every variant starts as a plain COPY TO of the original
gcContext = "copies"
USE (gcPercent) AGAIN SHARED ALIAS pfrx
COPY TO (gcVar + "v_all.frx")
COPY TO (gcVar + "v_noheader.frx")
COPY TO (gcVar + "v_platform.frx")
COPY TO (gcVar + "v_uniqueid.frx")
COPY TO (gcVar + "v_nofonts.frx")
COPY TO (gcVar + "v_node.frx")
COPY TO (gcVar + "v_timestamp.frx")
COPY TO (gcVar + "v_nouser.frx") FIELDS EXCEPT user
COPY TO (gcVar + "v_tencols.frx") FIELDS objtype, objcode, vpos, hpos, height, width, expr, picture, order, supexpr
COPY TO (gcVar + "v_noband.frx")
COPY TO (gcVar + "v_headerlast.frx") FOR objtype <> 1
COPY TO (gcVar + "v_platdos.frx")
COPY TO (gcVar + "v_fpt.dbf")
USE
fdvLog("COPY TO x.frx makes: frx " + TRANSFORM(FILE(gcVar + "v_all.frx")) + " frt " + TRANSFORM(FILE(gcVar + "v_all.frt")) + " fpt " + TRANSFORM(FILE(gcVar + "v_all.fpt")) ;
  + "; block size of the copy " + TRANSFORM(fdvBlockSize(gcVar + "v_all.frt")))
fdvLog("COPY TO x.dbf makes: dbf " + TRANSFORM(FILE(gcVar + "v_fpt.dbf")) + " fpt " + TRANSFORM(FILE(gcVar + "v_fpt.fpt")))

* v_all: every column, every record, untouched
fdvRun("v_all", "all 75 columns, all records, a plain copy")

* v_platform: PLATFORM blank everywhere
USE (gcVar + "v_platform.frx") EXCLUSIVE
REPLACE ALL platform WITH ""
USE
fdvRun("v_platform", "PLATFORM blank on every record")

* v_platdos: PLATFORM says DOS
USE (gcVar + "v_platdos.frx") EXCLUSIVE
REPLACE ALL platform WITH "DOS"
USE
fdvRun("v_platdos", "PLATFORM = DOS on every record")

* v_uniqueid: UNIQUEID blank everywhere
USE (gcVar + "v_uniqueid.frx") EXCLUSIVE
REPLACE ALL uniqueid WITH ""
USE
fdvRun("v_uniqueid", "UNIQUEID blank on every record")

* v_nofonts: the OBJTYPE 23 font resource records deleted
USE (gcVar + "v_nofonts.frx") EXCLUSIVE
DELETE FOR objtype = 23
PACK
USE
fdvRun("v_nofonts", "without the OBJTYPE 23 font records")

* v_node: the OBJTYPE 25 and 26 data environment records deleted; the tables it would have
* opened are opened here instead, with the variable its Init code would have made
USE (gcVar + "v_node.frx") EXCLUSIVE
DELETE FOR INLIST(objtype, 25, 26)
PACK
USE
OPEN DATABASE (gcRun + "data\testdata.dbc")
USE employee ORDER emp_id IN 0
USE orders ORDER emp_id IN 0
SELECT employee
SET RELATION TO emp_id INTO orders
PUBLIC nTotalSales
CALCULATE SUM(orders.order_amt) TO nTotalSales
fdvRun("v_node", "without the OBJTYPE 25/26 data environment records, tables opened by the program")
CLOSE DATABASES ALL

* v_timestamp: TIMESTAMP zero everywhere
USE (gcVar + "v_timestamp.frx") EXCLUSIVE
REPLACE ALL timestamp WITH 0
USE
fdvRun("v_timestamp", "TIMESTAMP 0 on every record")

* v_nouser: 74 columns, USER dropped
fdvRun("v_nouser", "74 columns: USER dropped")

* v_tencols: only the ten columns the runtime reads today
fdvRun("v_tencols", "only OBJTYPE OBJCODE VPOS HPOS HEIGHT WIDTH EXPR PICTURE ORDER SUPEXPR")

* v_noband: the page header band record deleted (its objects then sit in no band)
USE (gcVar + "v_noband.frx") EXCLUSIVE
DELETE FOR objtype = 9 AND objcode = 1
PACK
USE
fdvRun("v_noband", "without the page header band record")

* v_headerlast: the same records with the OBJTYPE 1 record moved to the end
USE (gcVar + "v_headerlast.frx") EXCLUSIVE
APPEND FROM (gcPercent) FOR objtype = 1
USE
fdvRun("v_headerlast", "OBJTYPE 1 record last instead of first")

* what a copy has in its header after PACK: the byte VFP writes at 0 and the flags
fdvLog("v_all header bytes: " + fdvHeaderBytes(gcVar + "v_all.frx"))
fdvLog("original header bytes: " + fdvHeaderBytes(gcPercent))

* v_noheader last of all: a report file without its OBJTYPE 1 record stops Visual FoxPro in a
* dialog the harness cannot see past (measured: the run had to be killed, with asked.txt
* written), so everything else is measured before this one is tried.
USE (gcVar + "v_noheader.frx") EXCLUSIVE
DELETE FOR objtype = 1
PACK
USE
fdvLog("v_noheader header bytes: " + fdvHeaderBytes(gcVar + "v_noheader.frx"))
fdvLog("about to run v_noheader, the copy without its OBJTYPE 1 record")
fdvRun("v_noheader", "without the OBJTYPE 1 header record")
gcContext = "end"
fdvLog("done")
RETURN

PROCEDURE fdvRun
LPARAMETERS cName, cWhat
LOCAL cOut, cText, oErr
cOut = gcRun + "fdv-v-" + cName + ".txt"
gcContext = cName
fdvLog("--- " + cName + ": " + cWhat)
* TRY around the statement rather than the ON ERROR handler, because a REPORT FORM that refuses
* a file does not always reach ON ERROR (measured: five variants wrote no file and logged no
* error), and CATCH says whether there was an exception at all. asked.txt is the harness's own
* mark: its keeper writes it whenever Visual FoxPro stops to wait for a person, so erasing it
* before the statement and looking for it afterwards says whether this variant put up a dialog.
ERASE (gcRun + "asked.txt")
TRY
  REPORT FORM (gcVar + cName + ".frx") TO FILE (cOut) ASCII NOCONSOLE
CATCH TO oErr
  fdvLog(cName + " raised error " + TRANSFORM(oErr.ErrorNo) + " [" + oErr.Message + "] line " + TRANSFORM(oErr.LineNo))
ENDTRY
IF FILE(gcRun + "asked.txt")
  fdvLog(cName + " stopped to ask a person; the harness answered with ESC")
ENDIF
IF FILE(cOut)
  cText = FILETOSTR(cOut)
  fdvLog(cName + " ran: " + TRANSFORM(LEN(cText)) + " bytes, " + IIF(cText == gcBaseline, "identical to the original", "DIFFERENT from the original"))
  IF cText == gcBaseline
    ERASE (cOut)
  ENDIF
ELSE
  fdvLog(cName + " wrote no file")
ENDIF
ENDPROC

PROCEDURE fdvLog
LPARAMETERS cText
STRTOFILE(cText + CHR(13) + CHR(10), gcRun + "fdv-log.txt", .T.)
ENDPROC

PROCEDURE fdvErr
LPARAMETERS nCode, cMsg, nLine, cProg
STRTOFILE("ERROR " + TRANSFORM(nCode) + " [" + cMsg + "] line " + TRANSFORM(nLine) + " in " + cProg + " during " + gcContext + CHR(13) + CHR(10), gcRun + "fdv-log.txt", .T.)
RETURN

* A DOS packed date and time: year-1980 in the top 7 bits, month 4, day 5, hour 5, minute 6,
* seconds/2 in the low 5. Written here to see whether the file's values read as sane dates.
FUNCTION fdvDosDate
LPARAMETERS nStamp
LOCAL nYear, nMonth, nDay, nHour, nMin, nSec
nYear = 1980 + INT(nStamp / 33554432)
nMonth = BITAND(INT(nStamp / 2097152), 15)
nDay = BITAND(INT(nStamp / 65536), 31)
nHour = BITAND(INT(nStamp / 2048), 31)
nMin = BITAND(INT(nStamp / 32), 63)
nSec = BITAND(nStamp, 31) * 2
RETURN TRANSFORM(nYear) + "-" + PADL(nMonth, 2, "0") + "-" + PADL(nDay, 2, "0") + " " + PADL(nHour, 2, "0") + ":" + PADL(nMin, 2, "0") + ":" + PADL(nSec, 2, "0")
ENDFUNC

FUNCTION fdvBlockSize
LPARAMETERS cFile
LOCAL h, c
IF NOT FILE(cFile)
  RETURN -1
ENDIF
h = FOPEN(cFile)
c = FREAD(h, 8)
FCLOSE(h)
RETURN ASC(SUBSTR(c, 7, 1)) * 256 + ASC(SUBSTR(c, 8, 1))
ENDFUNC

FUNCTION fdvHeaderBytes
LPARAMETERS cFile
LOCAL h, c, i, cHex
h = FOPEN(cFile)
c = FREAD(h, 32)
FCLOSE(h)
cHex = ""
FOR i = 1 TO 32
  cHex = cHex + RIGHT(TRANSFORM(ASC(SUBSTR(c, i, 1)), "@0"), 2) + " "
ENDFOR
RETURN cHex
ENDFUNC
