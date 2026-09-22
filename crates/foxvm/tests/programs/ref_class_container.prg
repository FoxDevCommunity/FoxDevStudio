* The controls that hold other controls: Container, PageFrame and Page.
LOCAL oBox, oFrame, oPage
oBox = CREATEOBJECT("Container")
? oBox.BaseClass, oBox.Class, oBox.Name, "[" + oBox.ParentClass + "]"
? TRANSFORM(oBox.Width), TRANSFORM(oBox.Height), TRANSFORM(oBox.BackColor), TRANSFORM(oBox.BackStyle)
? TRANSFORM(oBox.BorderColor), TRANSFORM(oBox.BorderWidth), TRANSFORM(oBox.SpecialEffect), TRANSFORM(oBox.ControlCount)
? oBox.Enabled, oBox.Visible, oBox.TabStop, "[" + oBox.Picture + "]", TRANSFORM(oBox.Style)

* a control added while it runs is one of its own, and it counts
oBox.AddObject("lblInside", "Label")
? TRANSFORM(oBox.ControlCount), oBox.lblInside.BaseClass, oBox.lblInside.Name
oBox.RemoveObject("lblInside")
? TRANSFORM(oBox.ControlCount)

? PEMSTATUS(oBox, "AddObject", 5), PEMSTATUS(oBox, "RemoveObject", 5), PEMSTATUS(oBox, "SetAll", 5)
? PEMSTATUS(oBox, "Draw", 5), PEMSTATUS(oBox, "NewObject", 5), PEMSTATUS(oBox, "Resize", 5)
? PEMSTATUS(oBox, "Caption", 5), PEMSTATUS(oBox, "FontName", 5), PEMSTATUS(oBox, "Themes", 5)

* a page frame holds pages, and starts out with none
oFrame = CREATEOBJECT("PageFrame")
? oFrame.BaseClass, oFrame.Name, TRANSFORM(oFrame.PageCount), TRANSFORM(oFrame.ActivePage)
? TRANSFORM(oFrame.Width), TRANSFORM(oFrame.Height), TRANSFORM(oFrame.PageHeight), TRANSFORM(oFrame.PageWidth)
? oFrame.Tabs, TRANSFORM(oFrame.TabStretch), TRANSFORM(oFrame.TabStyle), TRANSFORM(oFrame.TabOrientation)
? TRANSFORM(oFrame.BorderWidth), "[" + oFrame.MemberClass + "]", oFrame.RightToLeft
? PEMSTATUS(oFrame, "Pages", 5), PEMSTATUS(oFrame, "AddObject", 5), PEMSTATUS(oFrame, "SetAll", 5)

* a page is what a page frame holds
oPage = CREATEOBJECT("Page")
? oPage.BaseClass, oPage.Name, oPage.Caption, TRANSFORM(oPage.PageOrder), TRANSFORM(oPage.ControlCount)
? TRANSFORM(oPage.BackColor), TRANSFORM(oPage.BackStyle), oPage.FontName, TRANSFORM(oPage.FontSize)
? PEMSTATUS(oPage, "Activate", 5), PEMSTATUS(oPage, "Deactivate", 5), PEMSTATUS(oPage, "Width", 5)
* COVERS: Container, PageFrame, Page, BorderWidth, PageCount, ActivePage, PageHeight, PageWidth,
* COVERS: Tabs, TabStretch, TabStyle, TabOrientation, PageOrder, Pages, RemoveObject, NewObject,
* COVERS: Controls
