* p0-layout-rules: the Render log over every shipped report, and reports built to isolate a rule.
*
* A probe, not a golden: run it with
*   node scripts/vfp-expected.mjs --probe scripts/probes/reports/p0-layout-rules.prg
* Part one runs the shipped reports (and the community demo, when FDV_DEMO names a folder
* holding x.FRX and x.DBF) through a listener that logs every Render call to fdv-render.txt:
* page, record, left, top, width, height, continuation, contents. Five of them need a cursor
* their own program builds and are skipped with the reason; invoice.frx never finishes headless
* and is tried on its own at the end.
* Part two builds small reports record by record in a copy of percent.frx's structure, over a
* table made here, each isolating one rule: group change detection, totals reset timing, empty
* averages, page breaks, footers, columns, _PAGETOTAL, stretch, FLOAT/TOP/BOTTOM, print-when,
* NOREPEAT, SPACING, pictures, the band flags, OBJTYPE 10, and the data environment path rule.
* Those logs go to fdv-rules.txt (Render lines) and fdv-log.txt (what each variant is).
* Last of all come the four things that stop Visual FoxPro for good, each announced in the log
* first: a picture whose file is missing, a data environment whose database is missing,
* invoice.frx, and a field carrying NOREPEAT. A run ends at the first of them.
ON ERROR DO fdvErr WITH ERROR(), MESSAGE(), LINENO(), PROGRAM()
SET SAFETY OFF
SET TALK OFF
SET EXACT OFF
PUBLIC gcRun, gcContext, gcPercent, gcVar, gnTop, gnBandTop, gcRenderFile, gnErr
gcRun = ADDBS(SYS(5) + SYS(2003))
gcContext = "start"
gnErr = 0
gcPercent = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\Reports\percent.frx"
gcVar = gcRun + "sol\rep\"
gcRenderFile = gcRun + "fdv-render.txt"
LOCAL cSol, cDemo, i, n, ox, cName
LOCAL ARRAY aFiles[1]
cSol = "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Solution\"
fdvPrinter()

* ---------------------------------------------------------------- part one: the shipped reports
* A report whose data environment names a file that is not there, or which has no data
* environment and no table open, stops Visual FoxPro in an Open dialog the harness cannot see
* past (p0-frx-writer hung that way), so fdvSweep checks the files first and skips with a reason.
* Four of the shipped reports print a cursor their form or the Coverage Profiler builds
* (dbctofrx, ordgraph, MethodSummary, SlowLines) and are skipped; DynamicFormatting reads an
* orders table its program opens, and ColumnChartSample names no alias, so those two are run
* over a table opened here.
* the data the built reports' environments open, copied before anything opens a table of its own
gcContext = "data"
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

gcContext = "sweep"
fdvSweep(cSol + "Reports\percent.frx", "")
fdvSweep(cSol + "Reports\colors.frx", "")
fdvLog("sweep dbctofrx: skipped, it prints a cursor dbctofrx.scx builds")
fdvLog("sweep invoice: skipped here, it never finishes headless; tried on its own at the end")
fdvSweep(cSol + "Reports\ledger.frx", "")
fdvLog("sweep ordgraph: skipped, it prints a prodsales cursor ordgraph.scx builds")
fdvSweep(cSol + "Reports\wrapping.frx", "")
fdvSweep(cSol + "Coverage\Demos\report1.frx", "")
fdvLog("sweep MethodSummary: skipped, it prints a cursor the Coverage Profiler builds")
fdvLog("sweep SlowLines: skipped, it prints a cursor the Coverage Profiler builds")
fdvSweep(cSol + "Europa\ColumnChartSample.frx", "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Northwind\categories.dbf")
fdvSweep(cSol + "Europa\DynamicFormatting.frx", "C:\Program Files (x86)\Microsoft Visual FoxPro 9\Samples\Northwind\orders.dbf")
fdvSweep(cSol + "Europa\employeesmd.frx", "")
fdvSweep(cSol + "Europa\employeesmd2.frx", "")
* the community demo, as its own x.PRG runs it: REPORTBEHAVIOR 90, a listener of type 1 with
* DynamicLineHeight, over the x.DBF beside it
cDemo = GETENV("FDV_DEMO")
IF NOT EMPTY(cDemo) AND FILE(ADDBS(cDemo) + "x.frx")
  gcContext = "demo"
  CD (cDemo)
  USE x SHARED
  SET REPORTBEHAVIOR 90
  fdvDemo(ADDBS(cDemo) + "x.frx", 1, .T.)
  fdvDemo(ADDBS(cDemo) + "x.frx", 3, .F.)
  SET REPORTBEHAVIOR 80
  fdvDemo(ADDBS(cDemo) + "x.frx", 3, .F.)
  USE
  CD (gcRun)
ELSE
  fdvLog("demo: FDV_DEMO not set or x.frx absent, skipped")
ENDIF

* wrapping.frx without its two OBJTYPE 10 records: does the log change
gcContext = "objtype 10"
USE (cSol + "Reports\wrapping.frx") AGAIN SHARED ALIAS pw
COPY TO (gcVar + "w_with10.frx")
COPY TO (gcVar + "w_no10.frx") FOR objtype <> 10
USE
gcRenderFile = gcRun + "fdv-rules.txt"
fdvRun("w_with10", "wrapping.frx copied whole")
fdvRun("w_no10", "wrapping.frx without its OBJTYPE 10 records")
fdvLog("objtype 10: logs identical " + TRANSFORM(fdvLogOf("w_with10") == fdvLogOf("w_no10")))

* the data environment path rule: the same copy of percent from a folder where ..\..\data is
* only right when measured from the report's folder, then from one where only the default
* directory's ..\..\data is right
gcContext = "de path"
USE (gcPercent) AGAIN SHARED ALIAS pp
COPY TO (gcVar + "p_de.frx")
USE
MKDIR (gcRun + "other")
MKDIR (gcRun + "other\deep")
CD (gcRun)
fdvRun("p_de", "percent copy at sol\rep, default directory = run dir (run\..\..\data absent)")
CD (gcRun + "other\deep")
fdvRun("p_de", "percent copy at sol\rep, default directory = other\deep (other\deep\..\..\data present)")
CD (gcRun)

* ---------------------------------------------------------------- part two: built reports
gcContext = "build data"
CD (gcRun)
CREATE TABLE t (grp C(3), name C(12), val N(8,2), note C(200), pic G)
INSERT INTO t (grp, name, val, note) VALUES ("A", "one", 10.00, "short")
INSERT INTO t (grp, name, val, note) VALUES ("A", "two", 20.00, REPLICATE("wrap ", 40))
INSERT INTO t (grp, name, val, note) VALUES ("A ", "three", 30.00, "short")
INSERT INTO t (grp, name, val, note) VALUES ("a", "four", 40.00, "short")
INSERT INTO t (grp, name, val, note) VALUES ("B", "five", 50.00, "short")
INSERT INTO t (grp, name, val, note) VALUES ("B", "five", 60.00, "short")
INSERT INTO t (grp, name, val, note) VALUES ("C", "seven", 70.00, "short")
GO 1
APPEND GENERAL pic FROM (gcRun + "fdvfox.bmp")
USE

* g_plain: page header, group header/footer on grp, detail, page footer, summary
gcContext = "g_plain"
fdvNew("g_plain")
fdvBand(1, 3000, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(3, 2500, "grp")
fdvObj(8, 0, 0, 1875, 20000, '"Group " + grp', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, "val", "")
fdvObj(8, 0, 20000, 1875, 8000, "val", "")
REPLACE totaltype WITH 2, resettotal WITH 6
fdvBand(5, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, '"sum " + grp', "")
fdvObj(8, 0, 10000, 1875, 8000, "val", "")
REPLACE totaltype WITH 2, resettotal WITH 6
fdvObj(8, 0, 20000, 1875, 8000, "val", "")
REPLACE totaltype WITH 1, resettotal WITH 6
fdvObj(8, 0, 30000, 1875, 8000, "val", "")
REPLACE totaltype WITH 3, resettotal WITH 6
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO) + " of " + TRANSFORM(_PAGETOTAL)', "")
fdvBand(8, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, '"total"', "")
fdvObj(8, 0, 10000, 1875, 8000, "val", "")
REPLACE totaltype WITH 2, resettotal WITH 1
fdvObj(8, 0, 20000, 1875, 8000, "val", "")
REPLACE totaltype WITH 1, resettotal WITH 1
fdvObj(8, 0, 30000, 1875, 8000, "val", "")
REPLACE totaltype WITH 3, resettotal WITH 1
fdvObj(8, 0, 40000, 1875, 8000, "name", "")
REPLACE totaltype WITH 1, resettotal WITH 1
fdvFinish()
USE t
SET EXACT OFF
fdvRun("g_plain", "groups on grp with SET EXACT OFF; values A, A, A-with-blank, a, B, B, C; sum/count/avg per group and total; COUNT of a character field in the summary")
SET EXACT ON
fdvRun("g_plain", "the same with SET EXACT ON")
SET EXACT OFF
fdvRun("g_plain", "FOR .F.: no records at all; what AVERAGE and COUNT print", "FOR .F.")
fdvRun("g_plain", "FOR val > 45: totals under a FOR", "FOR val > 45")
fdvRun("g_plain", "SUMMARY clause", "SUMMARY")
fdvRun("g_plain", "with TwoPassProcess on the listener", "", .T.)
USE

* g_reset: the detail's running SUM reset at 1 (report), 2 (page), 3 (column)
FOR i = 1 TO 3
  gcContext = "g_reset" + TRANSFORM(i)
  fdvNew("g_reset" + TRANSFORM(i))
  fdvBand(1, 2500, "")
  fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
  fdvBand(3, 2500, "grp")
  fdvObj(8, 0, 0, 1875, 20000, '"Group " + grp', "")
  fdvBand(4, 2500, "")
  fdvObj(8, 0, 0, 1875, 8000, "name", "")
  fdvObj(8, 0, 10000, 1875, 8000, "val", "")
  REPLACE totaltype WITH 2, resettotal WITH i
  fdvBand(5, 2500, "")
  fdvObj(8, 0, 0, 1875, 8000, '"sum " + grp', "")
  fdvObj(8, 0, 10000, 1875, 8000, "val", "")
  REPLACE totaltype WITH 2, resettotal WITH i
  fdvBand(7, 2500, "")
  fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
  fdvFinish()
ENDFOR
USE t
fdvRun("g_reset1", "running SUM with RESETTOTAL 1 (end of report)")
fdvRun("g_reset2", "running SUM with RESETTOTAL 2 (end of page)")
fdvRun("g_reset3", "running SUM with RESETTOTAL 3 (end of column)")
USE

* b_break: a tall detail on a short page, so the page fills; the footer's place on every page
gcContext = "b_break"
fdvNew("b_break")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 20000, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 18000, 0, 1875, 8000, '"bottom of detail"', "")
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO) + " of " + TRANSFORM(_PAGETOTAL)', "")
fdvFinish()
USE t
fdvRun("b_break", "detail 2 inches tall, seven records: where the page breaks, where the footer sits")
USE

* b_tall: a detail taller than the page
gcContext = "b_tall"
fdvNew("b_tall")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 120000, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 60000, 0, 1875, 8000, '"middle"', "")
fdvObj(8, 118000, 0, 1875, 8000, '"bottom"', "")
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvRun("b_tall", "detail 12 inches tall on an 11 inch page, two records", "NEXT 2")
USE

* c_cols: two columns across the page, with column header and footer bands
gcContext = "c_cols"
fdvNew("c_cols")
USE (gcVar + "c_cols.frx") EXCLUSIVE ALIAS vfrx
GO 1
REPLACE vpos WITH 2, width WITH 35000, hpos WITH 5000, height WITH 0
USE
fdvOpen("c_cols")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(2, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Column header"', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, "val", "")
REPLACE totaltype WITH 2, resettotal WITH 3
fdvBand(6, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Column footer"', "")
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvRun("c_cols", "header VPOS 2 WIDTH 35000 HPOS 5000: two columns; column header and footer; SUM reset per column")
USE

* s_stretch: a stretching field over a long value, and what sits below it
gcContext = "s_stretch"
fdvNew("s_stretch")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 6000, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 15000, "note", "")
REPLACE stretch WITH .T.
fdvObj(8, 2500, 10000, 1875, 15000, '"float below"', "")
REPLACE float WITH .T., top WITH .F.
fdvObj(8, 2500, 30000, 1875, 15000, '"top below"', "")
REPLACE float WITH .F., top WITH .T.
fdvObj(8, 2500, 50000, 1875, 15000, '"bottom below"', "")
REPLACE float WITH .F., top WITH .F., bottom WITH .T.
fdvObj(6, 4500, 0, 104, 60000, "", "")
REPLACE float WITH .T., top WITH .F.
fdvObj(7, 0, 62000, 4000, 8000, "", "")
REPLACE float WITH .F., top WITH .T., bottom WITH .T.
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvRun("s_stretch", "STRETCH field over a 200 character note, with FLOAT, TOP and BOTTOM objects below it; a floating line; a rectangle marked top and bottom", "NEXT 3")
fdvRun("s_stretch", "the same with DynamicLineHeight = .T.", "NEXT 3", .F., .T.)
USE

* l_spacing: a two line label with SPACING 0, 1 and 2, and a band with DOUBLE
gcContext = "l_spacing"
fdvNew("l_spacing")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 8000, "")
fdvObj(5, 0, 0, 3750, 8000, '"line one" + CHR(13) + "line two"', "")
REPLACE spacing WITH 0
fdvObj(5, 0, 10000, 3750, 8000, '"line one" + CHR(13) + "line two"', "")
REPLACE spacing WITH 1
fdvObj(5, 0, 20000, 3750, 8000, '"line one" + CHR(13) + "line two"', "")
REPLACE spacing WITH 2
fdvObj(8, 0, 30000, 3750, 8000, "name", "")
REPLACE spacing WITH 2
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvRun("l_spacing", "two line labels with SPACING 0, 1, 2 and a field with SPACING 2", "NEXT 2")
USE (gcVar + "l_spacing.frx") EXCLUSIVE ALIAS vfrx
REPLACE ALL double WITH .T. FOR objtype = 9 AND objcode = 4
USE
USE t
fdvRun("l_spacing", "the same with DOUBLE on the detail band", "NEXT 2")
USE

* p_pic: pictures from a file, relative and absolute, and from a general field
gcContext = "p_pic"
COPY FILE (gcRun + "fdvfox.bmp") TO (gcVar + "fdvfox.bmp")
fdvNew("p_pic")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 12000, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(17, 0, 10000, 10000, 10000, "", "fdvfox.bmp")
REPLACE offset WITH 0, general WITH 0
fdvObj(17, 0, 22000, 10000, 10000, "", gcRun + "fdvfox.bmp")
REPLACE offset WITH 0, general WITH 1
fdvObj(17, 0, 34000, 10000, 10000, "", "")
REPLACE offset WITH 1, general WITH 1, name WITH "t.pic"
fdvObj(17, 0, 46000, 10000, 10000, "", "")
REPLACE offset WITH 2, general WITH 2, name WITH 'gcRun + "fdvfox.bmp"'
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
CD (gcRun)
fdvRun("p_pic", "pictures: relative file (in the report's folder, not the default directory), absolute file, general field t.pic, an expression", "NEXT 1")
ERASE (gcVar + "fdvfox.bmp")
fdvRun("p_pic", "the same with the relative file now only in the default directory", "NEXT 1")
CD (gcRun + "sol\rep")
fdvRun("p_pic", "the same with the default directory = the report's folder, no relative file anywhere", "NEXT 1")
CD (gcRun)
USE

* f_flags: each band flag on its own, on a group header, group footer, title and summary band
gcContext = "f_flags"
LOCAL cFlag, cBandName, nBand, k
LOCAL ARRAY aFlag[8], aBand[4]
aFlag[1] = "pagebreak"
aFlag[2] = "colbreak"
aFlag[3] = "resetpage"
aFlag[4] = "swapheader"
aFlag[5] = "swapfooter"
aFlag[6] = "ejectbefor"
aFlag[7] = "ejectafter"
aFlag[8] = "double"
aBand[1] = 0
aBand[2] = 3
aBand[3] = 5
aBand[4] = 8
fdvNew("f_base")
fdvBand(0, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Title"', "")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(3, 2500, "grp")
fdvObj(8, 0, 0, 1875, 20000, '"Group " + grp', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvBand(5, 2500, "")
fdvObj(8, 0, 0, 1875, 20000, '"end of " + grp', "")
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvBand(8, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Summary"', "")
fdvFinish()
USE t
fdvRun("f_base", "title, page header, group header, detail, group footer, page footer, summary; no flags")
FOR k = 1 TO 4
  nBand = aBand[k]
  FOR i = 1 TO 8
    cFlag = aFlag[i]
    cName = "f_" + TRANSFORM(nBand) + "_" + cFlag
    * in a work area of their own: a plain USE would close the table the report reports over
    USE (gcVar + "f_base.frx") AGAIN SHARED ALIAS pb IN 0
    SELECT pb
    COPY TO (gcVar + cName + ".frx")
    USE IN pb
    USE (gcVar + cName + ".frx") EXCLUSIVE ALIAS vfrx IN 0
    SELECT vfrx
    LOCATE FOR objtype = 9 AND objcode = nBand
    REPLACE &cFlag WITH .T.
    USE IN vfrx
    SELECT t
    fdvRun(cName, "band code " + TRANSFORM(nBand) + " with " + UPPER(cFlag))
  ENDFOR
ENDFOR
* the header record's PLAIN and SUMMARY columns (title and summary on their own page)
USE (gcVar + "f_base.frx") AGAIN SHARED ALIAS pb IN 0
SELECT pb
COPY TO (gcVar + "f_hdr_plain.frx")
COPY TO (gcVar + "f_hdr_summary.frx")
USE IN pb
USE (gcVar + "f_hdr_plain.frx") EXCLUSIVE ALIAS vfrx IN 0
SELECT vfrx
GO 1
REPLACE plain WITH .T.
USE IN vfrx
USE (gcVar + "f_hdr_summary.frx") EXCLUSIVE ALIAS vfrx IN 0
SELECT vfrx
GO 1
REPLACE summary WITH .T.
USE IN vfrx
SELECT t
fdvRun("f_hdr_plain", "header record PLAIN .T.")
fdvRun("f_hdr_summary", "header record SUMMARY .T.")
USE

* m_mode: a transparent and an opaque field over a filled rectangle (what Render is handed)
gcContext = "m_mode"
fdvNew("m_mode")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 4000, "")
fdvObj(7, 0, 0, 3500, 40000, "", "")
REPLACE fillpat WITH 1, fillred WITH 255, fillgreen WITH 255, fillblue WITH 0, penpat WITH 8, pensize WITH 1
fdvObj(8, 500, 1000, 1875, 8000, "name", "")
REPLACE mode WITH 1
fdvObj(8, 500, 12000, 1875, 8000, "name", "")
REPLACE mode WITH 0, fillred WITH 0, fillgreen WITH 255, fillblue WITH 255
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvRun("m_mode", "a filled rectangle drawn first in the band, a MODE 1 field and a MODE 0 field over it", "NEXT 1")
USE

* The print-when family, in four reports rather than one, because Visual FoxPro stopped in a
* dialog partway through the single one (measured) and everything after it went unmeasured.
* Each is announced before it runs, so the log says which of them the run died in.
gcContext = "w_supexpr"
fdvNew("w_supexpr")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 5000, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, "val", "")
REPLACE supexpr WITH "val > 35"
fdvObj(8, 2500, 10000, 1875, 8000, '"below the suppressed one"', "")
REPLACE float WITH .T., top WITH .F.
fdvObj(8, 2500, 0, 1875, 8000, '"below, not floating"', "")
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run w_supexpr")
fdvRun("w_supexpr", "SUPEXPR val > 35 on a field, with a floating field and a fixed one below it: whether a suppressed field leaves its space")
USE

gcContext = "w_supflags"
fdvNew("w_supflags")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, '"SUPALWAYS off"', "")
REPLACE supalways WITH .F.
fdvObj(8, 0, 20000, 1875, 8000, '"SUPOVFLOW on"', "")
REPLACE supovflow WITH .T.
fdvObj(8, 0, 30000, 1875, 8000, '"SUPRPCOL 0"', "")
REPLACE suprpcol WITH 0
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run w_supflags")
fdvRun("w_supflags", "SUPALWAYS .F., SUPOVFLOW .T. and SUPRPCOL 0, one field each")
USE

gcContext = "w_suprest"
fdvNew("w_suprest")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, '"SUPREST 1"', "")
REPLACE suprest WITH 1
fdvObj(8, 0, 20000, 1875, 8000, '"SUPTYPE 1"', "")
REPLACE suptype WITH 1
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run w_suprest, the one the single print-when report died in")
fdvRun("w_suprest", "SUPREST 1 on one field and SUPTYPE 1 on another")
USE

* last, because either may stop the run in a file dialog the harness cannot see past: a picture
* whose file is missing, then (p0-frx-writer hung this way) the database renamed away
gcContext = "picture missing"
fdvNew("p_nopic")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(4, 12000, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(17, 0, 10000, 10000, 10000, "", "nosuch.bmp")
REPLACE offset WITH 0, general WITH 0
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run p_nopic, a picture whose file does not exist")
fdvRun("p_nopic", "a picture record naming a file that does not exist", "NEXT 1")
USE
gcContext = "de missing"
fdvLog("about to run p_de with data\testdata.dbc renamed away")
RENAME (gcRun + "data\testdata.dbc") TO (gcRun + "data\testdata.dbx")
fdvRun("p_de", "percent copy at sol\rep, data\testdata.dbc renamed away")
RENAME (gcRun + "data\testdata.dbx") TO (gcRun + "data\testdata.dbc")

* invoice.frx last of all: its data environment opens the invoice view in testdata.dbc, and a
* headless run of it produced no Render in three minutes and had to be killed. Both forms are
* tried here, each announced first, so the log says which of them the run died in.
gcContext = "invoice"
CD (JUSTPATH(gcPercent))
fdvLog("about to run invoice.frx TO FILE ASCII")
REPORT FORM (JUSTPATH(gcPercent) + "\invoice.frx") TO FILE (gcRun + "fdv-invoice.txt") ASCII NOCONSOLE
fdvLog("invoice.frx TO FILE ASCII returned; file " + TRANSFORM(FILE(gcRun + "fdv-invoice.txt")))
fdvLog("about to run invoice.frx through the listener")
ox = CREATEOBJECT("fdvListener")
ox.cTag = "invoice"
ox.ListenerType = 3
ox.QuietMode = .T.
REPORT FORM (JUSTPATH(gcPercent) + "\invoice.frx") OBJECT ox NOCONSOLE
fdvLog("invoice.frx listener returned; pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders))
ox = .NULL.
CD (gcRun)
* NOREPEAT last of all: a field carrying it stopped Visual FoxPro in a dialog the harness
* could not answer, twice, so everything else is measured before these three are tried.
gcContext = "w_norepeat"
fdvNew("w_norepeat")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(3, 2500, "grp")
fdvObj(8, 0, 0, 1875, 20000, '"Group " + grp', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, "grp", "")
REPLACE norepeat WITH .T.
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run w_norepeat, which stopped the run in a dialog when its NOREPEAT field also carried SUPGROUP 1")
fdvRun("w_norepeat", "NOREPEAT on a field whose value repeats within a group")
USE

gcContext = "w_supgroup"
fdvNew("w_supgroup")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(3, 2500, "grp")
fdvObj(8, 0, 0, 1875, 20000, '"Group " + grp', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, "grp", "")
* 6 is the first group, the numbering RESETTOTAL uses; SUPGROUP 1 stopped the run in a dialog
REPLACE norepeat WITH .T., supgroup WITH 6
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run w_supgroup")
fdvRun("w_supgroup", "NOREPEAT with SUPGROUP 6, the first group")
USE

gcContext = "w_supvalchng"
fdvNew("w_supvalchng")
fdvBand(1, 2500, "")
fdvObj(5, 0, 0, 1875, 20000, '"Page header"', "")
fdvBand(3, 2500, "grp")
fdvObj(8, 0, 0, 1875, 20000, '"Group " + grp', "")
fdvBand(4, 2500, "")
fdvObj(8, 0, 0, 1875, 8000, "name", "")
fdvObj(8, 0, 10000, 1875, 8000, "grp", "")
REPLACE norepeat WITH .T., supvalchng WITH .T.
fdvBand(7, 2500, "")
fdvObj(8, 0, 0, 1875, 12000, '"Page " + TRANSFORM(_PAGENO)', "")
fdvFinish()
USE t
fdvLog("about to run w_supvalchng, which stopped the run in a dialog when it shared a report with the other NOREPEAT fields")
fdvRun("w_supvalchng", "NOREPEAT with SUPVALCHNG on one field")
USE

gcContext = "end"
fdvLog("done")
RETURN

* ------------------------------------------------------------------------------ the sweep
PROCEDURE fdvSweep
LPARAMETERS cFrx, cTable
LOCAL ox, cWas, cMissing
cWas = ADDBS(SYS(5) + SYS(2003))
gcContext = "sweep " + JUSTFNAME(cFrx)
cMissing = fdvDeMissing(cFrx)
IF NOT EMPTY(cMissing)
  fdvLog("sweep " + JUSTSTEM(cFrx) + ": skipped, its data environment names " + cMissing + " which is not there")
  RETURN
ENDIF
CD (JUSTPATH(cFrx))
IF NOT EMPTY(cTable)
  USE (cTable) SHARED
ENDIF
ox = CREATEOBJECT("fdvListener")
ox.cTag = JUSTSTEM(cFrx)
ox.ListenerType = 3
ox.QuietMode = .T.
* a shipped report can run to thousands of Render calls; the first of them show the shape
ox.nLogMax = 150
REPORT FORM (cFrx) OBJECT ox NOCONSOLE
fdvLog("sweep " + JUSTSTEM(cFrx) + ": pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders) ;
  + " page " + TRANSFORM(ox.nPageW) + "x" + TRANSFORM(ox.nPageH) + " _PAGETOTAL " + TRANSFORM(_PAGETOTAL))
ox = .NULL.
* a data environment leaves its tables open, and the next thing to copy or open them would
* find them in use (measured: error 3)
CLOSE DATABASES ALL
CD (cWas)
ENDPROC

* The community demo the way its own x.PRG runs it.
PROCEDURE fdvDemo
LPARAMETERS cFrx, nType, lDynamic
LOCAL ox
ox = CREATEOBJECT("fdvListener")
ox.cTag = "demo" + TRANSFORM(nType) + IIF(lDynamic, "dyn", "")
ox.ListenerType = nType
ox.QuietMode = .T.
ox.AllowModalMessages = .T.
ox.PrintJobName = "demo"
IF lDynamic
  ox.DynamicLineHeight = .T.
ENDIF
ox.nLogMax = 150
REPORT FORM (cFrx) OBJECT ox NOCONSOLE
fdvLog("demo at behaviour " + TRANSFORM(SET("REPORTBEHAVIOR")) + " ListenerType " + TRANSFORM(nType) + " DynamicLineHeight " + TRANSFORM(lDynamic) ;
  + ": pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders) + " page " + TRANSFORM(ox.nPageW) + "x" + TRANSFORM(ox.nPageH))
ox = .NULL.
ENDPROC

* The first file a report's data environment names that does not exist relative to the
* report's folder (the rule this probe measures with p_de), or "" when they are all there.
FUNCTION fdvDeMissing
LPARAMETERS cFrx
LOCAL cMissing, cDb, cSource, n, i, cFolder
LOCAL ARRAY aL[1]
cMissing = ""
cFolder = ADDBS(JUSTPATH(cFrx))
USE (cFrx) AGAIN SHARED ALIAS pde
SCAN FOR objtype = 26 AND ALLTRIM(LOWER(name)) == "cursor"
  cDb = ""
  cSource = ""
  n = ALINES(aL, expr)
  FOR i = 1 TO n
    IF LEFT(aL[i], 11) == "Database = "
      cDb = ALLTRIM(SUBSTR(aL[i], 12))
    ENDIF
    IF LEFT(aL[i], 15) == "CursorSource = "
      cSource = CHRTRAN(ALLTRIM(SUBSTR(aL[i], 16)), '"', "")
    ENDIF
  ENDFOR
  IF NOT EMPTY(cDb) AND NOT FILE(cFolder + cDb)
    cMissing = cDb
    EXIT
  ENDIF
  IF EMPTY(cDb) AND NOT EMPTY(cSource) AND NOT FILE(cFolder + FORCEEXT(cSource, "dbf"))
    cMissing = cSource
    EXIT
  ENDIF
ENDSCAN
USE
RETURN cMissing
ENDFUNC

* ------------------------------------------------------------------------- building reports
* A new report file in the run directory's sol\rep, with percent.frx's structure and its
* OBJTYPE 1 header record (letter paper, no printer environment), left open for fdvBand and
* fdvObj to append to. gnTop is where the next band starts; each band adds its height and the
* designer's separator bar, which is what places every object in percent.frx in its band.
PROCEDURE fdvNew
LPARAMETERS cName
USE (gcPercent) AGAIN SHARED ALIAS pfrx
COPY STRUCTURE TO (gcVar + cName + ".frx")
USE
USE (gcVar + cName + ".frx") EXCLUSIVE ALIAS vfrx
APPEND FROM (gcPercent) FOR objtype = 1
gnTop = 0
gnBandTop = 0
ENDPROC

PROCEDURE fdvOpen
LPARAMETERS cName
USE (gcVar + cName + ".frx") EXCLUSIVE ALIAS vfrx
gnTop = 0
gnBandTop = 0
ENDPROC

PROCEDURE fdvBand
LPARAMETERS nCode, nHeight, cExpr
APPEND BLANK
REPLACE platform WITH "WINDOWS", uniqueid WITH SYS(2015), objtype WITH 9, objcode WITH nCode, height WITH nHeight, expr WITH cExpr
gnBandTop = gnTop
gnTop = gnTop + nHeight + 10000 / 4.8
ENDPROC

* An object in the band last added, at nVpos below the band's top. Field defaults copied from
* percent.frx's own field records: Courier New 10, MODE 1, pen black, fill -1, SUPALWAYS,
* SUPRPCOL 3, TOP, RESETTOTAL 1.
PROCEDURE fdvObj
LPARAMETERS nType, nVpos, nHpos, nHeight, nWidth, cExpr, cPicture
APPEND BLANK
REPLACE platform WITH "WINDOWS", uniqueid WITH SYS(2015), objtype WITH nType, objcode WITH IIF(nType = 7, 4, 0), ;
  vpos WITH gnBandTop + nVpos, hpos WITH nHpos, height WITH nHeight, width WITH nWidth, expr WITH cExpr, picture WITH cPicture, ;
  fontface WITH "Courier New", fontsize WITH 10, fontstyle WITH 0, mode WITH 1, ;
  penred WITH 0, pengreen WITH 0, penblue WITH 0, fillred WITH -1, fillgreen WITH -1, fillblue WITH -1, ;
  supalways WITH .T., suprpcol WITH 3, top WITH .T., resettotal WITH IIF(nType = 8, 1, 0), spacing WITH 0
IF INLIST(nType, 6, 7)
  REPLACE mode WITH 0, pensize WITH 1, penpat WITH 8, penred WITH -1, pengreen WITH -1, penblue WITH -1, fontface WITH "", fontsize WITH 0, offset WITH IIF(nType = 6, 1, 0)
ENDIF
IF nType = 17
  REPLACE mode WITH 0, fontface WITH "", fontsize WITH 0, penred WITH 0, pengreen WITH 0, penblue WITH 0, fillred WITH 0, fillgreen WITH 0, fillblue WITH 0
ENDIF
ENDPROC

PROCEDURE fdvFinish
APPEND FROM (gcPercent) FOR objtype = 23 AND ALLTRIM(fontface) == "Courier New"
USE
ENDPROC

* ------------------------------------------------------------------------- running them
PROCEDURE fdvRun
LPARAMETERS cName, cWhat, cClauses, lTwoPass, lDynamic
LOCAL ox, cWas
cWas = ADDBS(SYS(5) + SYS(2003))
gcContext = cName
fdvLog("--- " + cName + ": " + cWhat + IIF(EMPTY(cClauses), "", " [" + cClauses + "]"))
STRTOFILE("--- " + cName + ": " + cWhat + IIF(EMPTY(cClauses), "", " [" + cClauses + "]") + CHR(13) + CHR(10), gcRenderFile, .T.)
ox = CREATEOBJECT("fdvListener")
ox.cTag = cName
ox.ListenerType = 3
ox.QuietMode = .T.
IF lTwoPass
  ox.TwoPassProcess = .T.
ENDIF
IF lDynamic
  ox.DynamicLineHeight = .T.
ENDIF
* from the top every time, so that a scope of NEXT n covers the same records in every run
IF NOT EMPTY(ALIAS())
  GO TOP
ENDIF
IF EMPTY(cClauses)
  REPORT FORM (gcVar + cName + ".frx") OBJECT ox NOCONSOLE
ELSE
  * a scope, FOR or SUMMARY goes where the syntax puts it, right after the file name
  REPORT FORM (gcVar + cName + ".frx") &cClauses OBJECT ox NOCONSOLE
ENDIF
fdvLog(cName + ": pages " + TRANSFORM(ox.OutputPageCount) + " renders " + TRANSFORM(ox.nRenders) + " page " + TRANSFORM(ox.nPageW) + "x" + TRANSFORM(ox.nPageH) ;
  + " _PAGENO " + TRANSFORM(_PAGENO) + " _PAGETOTAL " + TRANSFORM(_PAGETOTAL) + " PageTotal " + TRANSFORM(ox.PageTotal))
ox = .NULL.
CD (cWas)
ENDPROC

FUNCTION fdvLogOf
LPARAMETERS cName
LOCAL cAll, nAt, nEnd
cAll = FILETOSTR(gcRenderFile)
nAt = AT("--- " + cName + ":", cAll)
nEnd = AT("--- ", cAll, OCCURS("--- ", LEFT(cAll, nAt)) + 1)
IF nEnd = 0
  nEnd = LEN(cAll) + 1
ENDIF
RETURN STRTRAN(SUBSTR(cAll, nAt, nEnd - nAt), cName + " ", "")
ENDFUNC

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
  nLogMax = 0

  PROCEDURE BeforeReport
    THIS.nPageW = THIS.GetPageWidth()
    THIS.nPageH = THIS.GetPageHeight()
    DODEFAULT()
  ENDPROC

  PROCEDURE Render
    LPARAMETERS nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage
    THIS.nRenders = THIS.nRenders + 1
    IF THIS.nLogMax > 0 AND THIS.nRenders > THIS.nLogMax
      IF THIS.nRenders = THIS.nLogMax + 1
        STRTOFILE(THIS.cTag + " ... the rest of this report's Render calls are not logged" + CHR(13) + CHR(10), gcRenderFile, .T.)
      ENDIF
      DODEFAULT(nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage)
      RETURN
    ENDIF
    STRTOFILE(THIS.cTag + " P" + TRANSFORM(THIS.PageNo) + " R" + TRANSFORM(nFRXRecNo) + " " + TRANSFORM(nLeft) + " " + TRANSFORM(nTop) ;
      + " " + TRANSFORM(nWidth) + " " + TRANSFORM(nHeight) + " c" + TRANSFORM(nObjectContinuationType) + " img" + TRANSFORM(GDIPlusImage) ;
      + " [" + LEFT(CHRTRAN(cContentsToBeRendered, CHR(13) + CHR(10), "|"), 120) + "]" + CHR(13) + CHR(10), gcRenderFile, .T.)
    DODEFAULT(nFRXRecNo, nLeft, nTop, nWidth, nHeight, nObjectContinuationType, cContentsToBeRendered, GDIPlusImage)
  ENDPROC
ENDDEFINE
