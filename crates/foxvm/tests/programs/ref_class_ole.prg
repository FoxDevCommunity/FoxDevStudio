* The two OLE containers: the one a form holds something in, and the one bound to a General field.
*
* Neither can be made on its own - `CREATEOBJECT("OleControl")` is error 11 - so both are added
* to a form, which is the only place one lives. The container needs something to hold, and what
* it holds decides what else it answers to: this one holds a Paint picture, which is a document
* rather than an ActiveX control, and a document container is the one the reference describes.
LOCAL oFrm
oFrm = CREATEOBJECT("Form")
oFrm.AddObject("oleDoc", "OleControl", "Paint.Picture")
oFrm.AddObject("oleFld", "OleBoundControl")

? oFrm.oleDoc.BaseClass, oFrm.oleDoc.Class, oFrm.oleDoc.Name, "[" + oFrm.oleDoc.ParentClass + "]"
? TRANSFORM(oFrm.oleDoc.Width), TRANSFORM(oFrm.oleDoc.Height), TRANSFORM(oFrm.oleDoc.Left), TRANSFORM(oFrm.oleDoc.Top)
? TRANSFORM(oFrm.oleDoc.AutoActivate), oFrm.oleDoc.AutoVerbMenu, oFrm.oleDoc.AutoSize, oFrm.oleDoc.Sizable
? TRANSFORM(oFrm.oleDoc.OLETypeAllowed), TRANSFORM(oFrm.oleDoc.OLELCID), TRANSFORM(oFrm.oleDoc.Stretch)
? "[" + oFrm.oleDoc.HostName + "]", "[" + oFrm.oleDoc.DocumentFile + "]", oFrm.oleDoc.Enabled, oFrm.oleDoc.Visible

? oFrm.oleFld.BaseClass, oFrm.oleFld.Class, oFrm.oleFld.Name
? TRANSFORM(oFrm.oleFld.AutoActivate), oFrm.oleFld.AutoVerbMenu, "[" + oFrm.oleFld.ControlSource + "]"
? TRANSFORM(oFrm.oleFld.OLETypeAllowed), TRANSFORM(oFrm.oleFld.WhatsThisHelpID), "[" + oFrm.oleFld.OLEClass + "]"
? "[" + oFrm.oleFld.StatusBarText + "]", "[" + oFrm.oleFld.ToolTipText + "]"

* what a container can be asked to do, and what it is told
? PEMSTATUS(oFrm.oleDoc, "DoVerb", 5), PEMSTATUS(oFrm.oleDoc, "OLEClass", 5), PEMSTATUS(oFrm.oleDoc, "Refresh", 5)
? PEMSTATUS(oFrm.oleDoc, "Moved", 5), PEMSTATUS(oFrm.oleDoc, "Resize", 5), PEMSTATUS(oFrm.oleDoc, "UIEnable", 5)
? PEMSTATUS(oFrm.oleFld, "DoVerb", 5), PEMSTATUS(oFrm.oleFld, "ShowWhatsThis", 5), PEMSTATUS(oFrm.oleFld, "ZOrder", 5)

* a container is not a control group: nothing of a form's own is on it
? PEMSTATUS(oFrm.oleDoc, "Caption", 5), PEMSTATUS(oFrm.oleDoc, "BackColor", 5), PEMSTATUS(oFrm.oleDoc, "Click", 5)
* COVERS: OLE Container, OLE Bound, AutoActivate, AutoVerbMenu, DocumentFile, HostName, OLEClass,
* COVERS: OLELCID, OLETypeAllowed, DoVerb
