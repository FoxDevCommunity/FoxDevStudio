* The three controls a person types into: TextBox, EditBox and Spinner.
LOCAL o, oEdit, oSpin
o = CREATEOBJECT("TextBox")
? o.BaseClass, o.Class, o.Name, "[" + o.ParentClass + "]"
? "[" + o.Value + "]", "[" + o.ControlSource + "]", "[" + o.InputMask + "]", "[" + o.Format + "]"
? TRANSFORM(o.Width), TRANSFORM(o.Height), TRANSFORM(o.Alignment), TRANSFORM(o.Margin)
? TRANSFORM(o.BackColor), TRANSFORM(o.ForeColor), TRANSFORM(o.BorderColor), TRANSFORM(o.BorderStyle)
? TRANSFORM(o.SelectedBackColor), TRANSFORM(o.SelectedForeColor), TRANSFORM(o.ColorScheme)
? o.ReadOnly, o.SelectOnEntry, o.HideSelection, TRANSFORM(o.MaxLength), o.EnableHyperlinks
? TRANSFORM(o.SelStart), TRANSFORM(o.SelLength), "[" + o.SelText + "]", TRANSFORM(LEN(o.Text))
? TRANSFORM(o.Century), TRANSFORM(o.DateFormat), "[" + o.DateMark + "]", TRANSFORM(o.Hours)
? TRANSFORM(o.Seconds), TRANSFORM(o.StrictDateEntry), TRANSFORM(o.IMEMode), "[" + o.NullDisplay + "]"
? TRANSFORM(o.AutoComplete), "[" + o.AutoCompSource + "]", "[" + o.AutoCompTable + "]"
? "[" + o.PasswordChar + "]", o.IntegralHeight, o.OpenWindow, "[" + o.MemoWindow + "]"

o.Value = "Ada"
o.ReadOnly = .T.
o.MaxLength = 30
? "[" + o.Value + "]", o.ReadOnly, TRANSFORM(o.MaxLength)

* what a text box answers to: the two change events, the focus pair, and no Print of its own
? PEMSTATUS(o, "InteractiveChange", 5), PEMSTATUS(o, "ProgrammaticChange", 5), PEMSTATUS(o, "KeyPress", 5)
? PEMSTATUS(o, "RangeHigh", 5), PEMSTATUS(o, "RangeLow", 5), PEMSTATUS(o, "UIEnable", 5)
? PEMSTATUS(o, "SetFocus", 5), PEMSTATUS(o, "Refresh", 5), PEMSTATUS(o, "Print", 5)

* an edit box is a text box that holds lines, so it has scroll bars and no input mask
oEdit = CREATEOBJECT("EditBox")
? oEdit.BaseClass, oEdit.Name, TRANSFORM(oEdit.Width), TRANSFORM(oEdit.Height), TRANSFORM(oEdit.ScrollBars)
? oEdit.AllowTabs, oEdit.AddLineFeeds, TRANSFORM(oEdit.ColorScheme), "[" + oEdit.Value + "]"
? PEMSTATUS(oEdit, "InputMask", 5), PEMSTATUS(oEdit, "AllowTabs", 5), PEMSTATUS(oEdit, "ScrollBars", 5)

* a spinner counts, so it has an increment and the two clicks that work it
oSpin = CREATEOBJECT("Spinner")
? oSpin.BaseClass, oSpin.Name, TRANSFORM(oSpin.Value), TRANSFORM(oSpin.Increment), TRANSFORM(oSpin.Alignment)
? LTRIM(STR(oSpin.SpinnerHighValue, 12)), LTRIM(STR(oSpin.SpinnerLowValue, 12))
? LTRIM(STR(oSpin.KeyboardHighValue, 12)), LTRIM(STR(oSpin.KeyboardLowValue, 12))
? TRANSFORM(oSpin.Height), TRANSFORM(oSpin.Width)
oSpin.Value = 7
oSpin.Increment = 0.5
? TRANSFORM(oSpin.Value), TRANSFORM(oSpin.Increment)
? PEMSTATUS(oSpin, "UpClick", 5), PEMSTATUS(oSpin, "DownClick", 5), PEMSTATUS(oSpin, "RangeHigh", 5)
* COVERS: TextBox, EditBox, Spinner, InputMask, Format, Margin, ColorScheme, ReadOnly,
* COVERS: SelectOnEntry, HideSelection, MaxLength, EnableHyperlinks, SelStart, SelLength,
* COVERS: SelText, Text, Century, DateFormat, DateMark, Hours, Seconds, StrictDateEntry, IMEMode,
* COVERS: NullDisplay, AutoComplete, AutoCompSource, AutoCompTable, PasswordChar, IntegralHeight,
* COVERS: OpenWindow, MemoWindow, ScrollBars, AllowTabs, AddLineFeeds, Increment, RangeHigh,
* COVERS: RangeLow, UpClick, DownClick, KeyPress
