* The controls that answer a question: CheckBox, OptionGroup and OptionButton.
LOCAL oCheck, oOptions, oButton
oCheck = CREATEOBJECT("CheckBox")
? oCheck.BaseClass, oCheck.Class, oCheck.Name, "[" + oCheck.ParentClass + "]"
? oCheck.Caption, TRANSFORM(oCheck.Value), TRANSFORM(oCheck.Alignment), TRANSFORM(oCheck.Style)
? TRANSFORM(oCheck.Width), TRANSFORM(oCheck.Height), TRANSFORM(oCheck.BackColor), TRANSFORM(oCheck.BackStyle)
? oCheck.AutoSize, oCheck.WordWrap, oCheck.Centered, oCheck.ReadOnly, oCheck.Themes
? "[" + oCheck.ControlSource + "]", "[" + oCheck.Picture + "]", TRANSFORM(oCheck.PicturePosition)
? TRANSFORM(oCheck.SpecialEffect), TRANSFORM(oCheck.ColorScheme), TRANSFORM(oCheck.DisabledForeColor)

* a check box holds a number, and a logical written into it stays logical
oCheck.Value = 1
? TRANSFORM(oCheck.Value), VARTYPE(oCheck.Value)
oCheck.Value = .T.
? TRANSFORM(oCheck.Value), VARTYPE(oCheck.Value)
oCheck.Caption = "\<Loud"
? oCheck.Caption

? PEMSTATUS(oCheck, "InteractiveChange", 5), PEMSTATUS(oCheck, "ProgrammaticChange", 5)
? PEMSTATUS(oCheck, "Click", 5), PEMSTATUS(oCheck, "DblClick", 5), PEMSTATUS(oCheck, "BorderStyle", 5)

* an option group is a container of option buttons, and starts out with none of them
oOptions = CREATEOBJECT("OptionGroup")
? oOptions.BaseClass, oOptions.Name, TRANSFORM(oOptions.ButtonCount), TRANSFORM(oOptions.Value)
? TRANSFORM(oOptions.Width), TRANSFORM(oOptions.Height), TRANSFORM(oOptions.BorderStyle), TRANSFORM(oOptions.SpecialEffect)
? "[" + oOptions.ControlSource + "]", "[" + oOptions.MemberClass + "]", oOptions.AutoSize
? PEMSTATUS(oOptions, "AddObject", 5), PEMSTATUS(oOptions, "SetAll", 5), PEMSTATUS(oOptions, "Buttons", 5)
? PEMSTATUS(oOptions, "GotFocus", 5), PEMSTATUS(oOptions, "KeyPress", 5), PEMSTATUS(oOptions, "Click", 5)

* an option button is one of them, on its own
oButton = CREATEOBJECT("OptionButton")
? oButton.BaseClass, oButton.Name, oButton.Caption, TRANSFORM(oButton.Value)
? TRANSFORM(oButton.Width), TRANSFORM(oButton.Height), TRANSFORM(oButton.Alignment), oButton.AutoSize
? PEMSTATUS(oButton, "GotFocus", 5), PEMSTATUS(oButton, "InteractiveChange", 5), PEMSTATUS(oButton, "SetFocus", 5)
* COVERS: CheckBox, OptionGroup, OptionButton, Centered, Style, BackStyle, Buttons
