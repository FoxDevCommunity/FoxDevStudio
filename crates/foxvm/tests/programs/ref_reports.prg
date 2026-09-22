* A report run over a table, and what a program can ask about the printer it would go to.
CREATE TABLE parts (code C(8), name C(10), price N(8,2))
INSERT INTO parts VALUES ("A1", "Bolt", 2.50)
INSERT INTO parts VALUES ("B2", "Nut", 1.75)
INSERT INTO parts VALUES ("C3", "Washer", 0.40)

* the whole thing, every band in order
REPORT FORM parts

* the records it covers, and the bands it leaves out
GO TOP
REPORT FORM parts FOR price > 1
REPORT FORM parts SUMMARY

* and where it goes instead of the screen
REPORT FORM parts TO FILE partlist.txt NOCONSOLE
? "written: " + IIF(FILE("partlist.txt"), "yes", "no")
REPORT FORM parts NOCONSOLE TO PRINTER
REPORT FORM parts PREVIEW NOCONSOLE
REPORT FORM parts PLAIN NOCONSOLE

* the page ends where the program says it does
EJECT

* what there is to print to
? PRINTSTATUS()
? PRTINFO(1), PRTINFO(2), PRTINFO(3)
DIMENSION aPrint(1)
? APRINTERS(aPrint)
? aPrint(1, 1)
? GETPRINTER()

* the listener the report engine hands each band to
oList = CREATEOBJECT("ReportListener")
? oList.Class
? oList.ListenerType, oList.CurrentPass, oList.TwoPassProcess
? oList.PageNo, oList.PageTotal, oList.OutputPageCount
? oList.QuietMode, oList.AllowModalMessages
oList.PrintJobName = "Part list"
? oList.PrintJobName
? oList.SupportsListenerType(1), oList.SupportsListenerType(9)
? oList.GetPageHeight(), oList.GetPageWidth()
? oList.IncludePageInOutput()
oList.CancelReport()
? oList.OutputPageCount

USE
DROP TABLE parts.dbf
ERASE partlist.txt

* COVERS: APRINTERS, AdjustObjectSize, AfterBand, AfterReport, AllowModalMessages,
* COVERS: BeforeBand, BeforeReport, CancelReport, CommandClauses, CurrentPass, EJECT,
* COVERS: EvaluateContents, FRXDataSession, GETPRINTER, GetPageHeight, GetPageWidth,
* COVERS: IncludePageInOutput, ListenerType, LoadReport, OutputPage, OutputPageCount,
* COVERS: OutputType, PRINTSTATUS, PRTINFO, PageHeight, PageNo, PageTotal, PageWidth,
* COVERS: PreviewContainer, PrintJobName, QuietMode, REPORT FORM, Render, SupportsListenerType,
* COVERS: TwoPassProcess, UnloadReport
