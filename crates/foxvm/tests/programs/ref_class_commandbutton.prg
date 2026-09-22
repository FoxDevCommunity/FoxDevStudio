* The CommandButton and CommandGroup base classes: what a new one holds and what it answers to.
LOCAL o, oGroup
o = CREATEOBJECT("CommandButton")
? o.BaseClass, o.Class, o.Name, "[" + o.ParentClass + "]"
? o.Caption, TRANSFORM(o.Alignment), o.AutoSize, o.WordWrap, o.Default, o.Cancel
? TRANSFORM(o.Left), TRANSFORM(o.Top), TRANSFORM(o.Width), TRANSFORM(o.Height)
? TRANSFORM(o.BackColor), TRANSFORM(o.ForeColor), TRANSFORM(o.DisabledBackColor), TRANSFORM(o.DisabledForeColor)
? "[" + o.Picture + "]", "[" + o.DownPicture + "]", "[" + o.DisabledPicture + "]"
? TRANSFORM(o.PicturePosition), TRANSFORM(o.PictureMargin), TRANSFORM(o.PictureSpacing)
? TRANSFORM(o.SpecialEffect), TRANSFORM(o.VisualEffect), TRANSFORM(o.Style), o.Themes
? o.Enabled, o.Visible, o.TabStop, TRANSFORM(o.TabIndex), o.TerminateRead
? "[" + o.StatusBarText + "]", "[" + o.ToolTipText + "]", TRANSFORM(o.WhatsThisHelpID)

o.Caption = "\<Save"
o.Default = .T.
o.Width = 84
? o.Caption, o.Default, TRANSFORM(o.Width)

* a button has a Click but no DblClick, and none of the methods a container has
? PEMSTATUS(o, "Click", 5), PEMSTATUS(o, "DblClick", 5), PEMSTATUS(o, "RightClick", 5)
? PEMSTATUS(o, "GotFocus", 5), PEMSTATUS(o, "LostFocus", 5), PEMSTATUS(o, "When", 5), PEMSTATUS(o, "Valid", 5)
? PEMSTATUS(o, "UIEnable", 5), PEMSTATUS(o, "Message", 5), PEMSTATUS(o, "ErrorMessage", 5)
? PEMSTATUS(o, "SetFocus", 5), PEMSTATUS(o, "Drag", 5), PEMSTATUS(o, "OLEDrag", 5)
? PEMSTATUS(o, "AddObject", 5), PEMSTATUS(o, "SetAll", 5), PEMSTATUS(o, "Value", 5)

* a CommandGroup is a container of buttons, and starts out with none
oGroup = CREATEOBJECT("CommandGroup")
? oGroup.BaseClass, oGroup.Name, TRANSFORM(oGroup.ButtonCount), TRANSFORM(oGroup.Value)
? TRANSFORM(oGroup.Width), TRANSFORM(oGroup.Height), TRANSFORM(oGroup.BackColor), TRANSFORM(oGroup.BorderColor)
? TRANSFORM(oGroup.BorderStyle), TRANSFORM(oGroup.SpecialEffect), "[" + oGroup.ControlSource + "]"
? "[" + oGroup.MemberClass + "]", "[" + oGroup.MemberClassLibrary + "]", oGroup.AutoSize
? PEMSTATUS(oGroup, "AddObject", 5), PEMSTATUS(oGroup, "SetAll", 5), PEMSTATUS(oGroup, "AddProperty", 5)
? PEMSTATUS(oGroup, "InteractiveChange", 5), PEMSTATUS(oGroup, "ProgrammaticChange", 5), PEMSTATUS(oGroup, "Click", 5)
* COVERS: CommandButton, CommandGroup, Cancel, Default, DownPicture, DisabledPicture, Picture,
* COVERS: PicturePosition, PictureMargin, PictureSpacing, SpecialEffect, VisualEffect, Themes,
* COVERS: TabStop, TerminateRead, ButtonCount, BorderColor, ControlSource, MemberClass,
* COVERS: MemberClassLibrary, SetAll, AddProperty, Drag, OLEDrag, When, Valid, UIEnable, Message,
* COVERS: ErrorMessage, GotFocus, LostFocus, RightClick, DblClick, SetFocus, InteractiveChange,
* COVERS: ProgrammaticChange, Value
