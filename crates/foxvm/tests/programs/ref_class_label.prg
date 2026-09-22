* The Label base class: what a new one says it is, what it holds, and what a write reads back.
*
* Numbers go through TRANSFORM so that the line does not depend on how wide a number prints.
LOCAL o
o = CREATEOBJECT("Label")
? o.BaseClass, o.Class, o.Name, "[" + o.ParentClass + "]"
? o.Caption, TRANSFORM(o.Alignment), o.AutoSize, o.WordWrap
? TRANSFORM(o.Left), TRANSFORM(o.Top), TRANSFORM(o.Width), TRANSFORM(o.Height)
? TRANSFORM(o.BackColor), TRANSFORM(o.ForeColor), TRANSFORM(o.BackStyle), TRANSFORM(o.BorderStyle)
? TRANSFORM(o.DisabledBackColor), TRANSFORM(o.DisabledForeColor), TRANSFORM(o.ColorSource)
? o.FontName, TRANSFORM(o.FontSize), o.FontBold, o.FontItalic, o.FontUnderline, o.FontStrikethru
? o.Enabled, o.Visible, "[" + o.ToolTipText + "]", TRANSFORM(o.MousePointer), TRANSFORM(o.Anchor)
? "[" + o.Comment + "]", "[" + o.Tag + "]", TRANSFORM(o.HelpContextID), TRANSFORM(o.WhatsThisHelpID)
? TRANSFORM(o.TabIndex), "[" + o.StatusBarText + "]", TRANSFORM(o.Style), TRANSFORM(o.Rotation)
? TRANSFORM(o.OLEDropEffects), TRANSFORM(o.OLEDropMode), TRANSFORM(o.DragMode), "[" + o.DragIcon + "]"
? TRANSFORM(o.OLEDragMode), "[" + o.OLEDragPicture + "]", TRANSFORM(o.OLEDropHasData), o.RightToLeft
? TRANSFORM(o.FontCharSet), o.FontOutline, o.FontShadow, "[" + o.MouseIcon + "]", "[" + o.ClassLibrary + "]"

* what a write reads back
o.Caption = "Your name:"
o.AutoSize = .T.
o.BackColor = 255
o.FontSize = 12
? o.Caption, o.AutoSize, TRANSFORM(o.BackColor), TRANSFORM(o.FontSize)

* a Label answers to the events of something you can click but not type in, and to none of the
* methods that belong to a container
? PEMSTATUS(o, "Click", 5), PEMSTATUS(o, "DblClick", 5), PEMSTATUS(o, "MouseMove", 5)
? PEMSTATUS(o, "MouseEnter", 5), PEMSTATUS(o, "MouseLeave", 5), PEMSTATUS(o, "MouseWheel", 5)
? PEMSTATUS(o, "Init", 5), PEMSTATUS(o, "Destroy", 5), PEMSTATUS(o, "Error", 5)
? PEMSTATUS(o, "Move", 5), PEMSTATUS(o, "ZOrder", 5), PEMSTATUS(o, "ResetToDefault", 5)
? PEMSTATUS(o, "AddObject", 5), PEMSTATUS(o, "SetFocus", 5), PEMSTATUS(o, "Value", 5)
? PEMSTATUS(o, "GotFocus", 5), PEMSTATUS(o, "InteractiveChange", 5), PEMSTATUS(o, "Refresh", 5)
? PEMSTATUS(o, "MouseDown", 5), PEMSTATUS(o, "MouseUp", 5), PEMSTATUS(o, "MiddleClick", 5), PEMSTATUS(o, "RightClick", 5)
? PEMSTATUS(o, "DragDrop", 5), PEMSTATUS(o, "DragOver", 5), PEMSTATUS(o, "Drag", 5), PEMSTATUS(o, "OLEDrag", 5)
? PEMSTATUS(o, "OLEStartDrag", 5), PEMSTATUS(o, "OLEDragOver", 5), PEMSTATUS(o, "OLEDragDrop", 5)
? PEMSTATUS(o, "OLEGiveFeedback", 5), PEMSTATUS(o, "OLESetData", 5), PEMSTATUS(o, "OLECompleteDrag", 5)
? PEMSTATUS(o, "ReadExpression", 5), PEMSTATUS(o, "WriteExpression", 5), PEMSTATUS(o, "ReadMethod", 5)
? PEMSTATUS(o, "WriteMethod", 5), PEMSTATUS(o, "SaveAsClass", 5), PEMSTATUS(o, "ShowWhatsThis", 5)
? PEMSTATUS(o, "AddProperty", 5), PEMSTATUS(o, "CloneObject", 5), PEMSTATUS(o, "SetAll", 5)
* COVERS: Label, Alignment, AutoSize, WordWrap, BackStyle, BorderStyle, ColorSource, FontName,
* COVERS: FontSize, ToolTipText, MousePointer, Anchor, Comment, Tag, HelpContextID,
* COVERS: WhatsThisHelpID, TabIndex, StatusBarText, Style, Rotation, OLEDropEffects, OLEDropMode,
* COVERS: DragMode, DragIcon, Caption, ParentClass, BaseClass, Class, Name, Left, Top, Width,
* COVERS: Height, Enabled, Visible, MouseEnter, MouseLeave, MouseWheel, ZOrder, ResetToDefault,
* COVERS: Click, DblClick, MouseMove, MouseDown, MouseUp, MiddleClick, RightClick, Init, Destroy,
* COVERS: Error, DragDrop, DragOver, Drag, OLEDrag, OLEStartDrag, OLEDragOver, OLEGiveFeedback,
* COVERS: OLESetData, OLECompleteDrag, ReadExpression, WriteExpression, ReadMethod, WriteMethod,
* COVERS: SaveAsClass, ShowWhatsThis, AddProperty, CloneObject, SetAll, Move, Refresh, SetFocus,
* COVERS: GotFocus, OLEDragMode, OLEDragPicture, OLEDropHasData, RightToLeft, FontCharSet,
* COVERS: FontOutline, FontShadow, MouseIcon, ClassLibrary
