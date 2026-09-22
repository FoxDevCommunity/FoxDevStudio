* The report listener: the object the report engine hands each band to as it lays a report out.
*
* `ref_reports.prg` runs one against a report; this is the class on its own - what a brand new
* one holds, and what it answers to.
LOCAL oList
oList = CREATEOBJECT("ReportListener")
? oList.BaseClass, oList.Class, oList.Name, "[" + oList.ParentClass + "]"
? TRANSFORM(oList.ListenerType), TRANSFORM(oList.OutputType), TRANSFORM(oList.CurrentPass)
? TRANSFORM(oList.CurrentDataSession), TRANSFORM(oList.FRXDataSession)
? oList.TwoPassProcess, oList.QuietMode, oList.AllowModalMessages, oList.DynamicLineHeight
? TRANSFORM(oList.GDIPlusGraphics), TRANSFORM(oList.SendGDIPlusImage)
? VARTYPE(oList.CommandClauses), VARTYPE(oList.PreviewContainer), "[" + oList.PrintJobName + "]"

* what it is told while a report runs, and what it can be asked to do
? PEMSTATUS(oList, "LoadReport", 5), PEMSTATUS(oList, "UnloadReport", 5), PEMSTATUS(oList, "BeforeReport", 5)
? PEMSTATUS(oList, "AfterReport", 5), PEMSTATUS(oList, "BeforeBand", 5), PEMSTATUS(oList, "AfterBand", 5)
? PEMSTATUS(oList, "EvaluateContents", 5), PEMSTATUS(oList, "AdjustObjectSize", 5), PEMSTATUS(oList, "Render", 5)
? PEMSTATUS(oList, "OutputPage", 5), PEMSTATUS(oList, "CancelReport", 5), PEMSTATUS(oList, "IncludePageInOutput", 5)
? PEMSTATUS(oList, "SupportsListenerType", 5), PEMSTATUS(oList, "GetPageHeight", 5), PEMSTATUS(oList, "GetPageWidth", 5)

* the four it says how a run is going through, and the one a preview window closing calls
? PEMSTATUS(oList, "DoStatus", 5), PEMSTATUS(oList, "UpdateStatus", 5), PEMSTATUS(oList, "ClearStatus", 5)
? PEMSTATUS(oList, "DoMessage", 5), PEMSTATUS(oList, "OnPreviewClose", 5)

* a listener is not a control: there is no drawing on it and nothing to click
? PEMSTATUS(oList, "Width", 5), PEMSTATUS(oList, "Visible", 5), PEMSTATUS(oList, "Click", 5)
* COVERS: ReportListener, CurrentDataSession, DynamicLineHeight, GDIPlusGraphics,
* COVERS: SendGDIPlusImage, ClearStatus, DoMessage, DoStatus, UpdateStatus, OnPreviewClose
