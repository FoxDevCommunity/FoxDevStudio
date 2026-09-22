* The Form base class: a window a program can make for itself, and what it starts out as.
*
* The window handle is left alone: two forms in one run already have different ones, so it is
* not a default and there is nothing to compare.
LOCAL oForm2
oForm2 = CREATEOBJECT("Form")
? oForm2.BaseClass, oForm2.Class, oForm2.Name, "[" + oForm2.ParentClass + "]"
? oForm2.Caption, TRANSFORM(oForm2.Width), TRANSFORM(oForm2.Height), TRANSFORM(oForm2.Left), TRANSFORM(oForm2.Top)
? TRANSFORM(oForm2.BackColor), TRANSFORM(oForm2.ForeColor), TRANSFORM(oForm2.BorderStyle), TRANSFORM(oForm2.TitleBar)
? oForm2.Visible, oForm2.Enabled, oForm2.Closable, oForm2.Movable, oForm2.ControlBox
? oForm2.MaxButton, oForm2.MinButton, oForm2.AutoCenter, oForm2.AlwaysOnTop, oForm2.AlwaysOnBottom
? TRANSFORM(oForm2.WindowState), TRANSFORM(oForm2.WindowType), TRANSFORM(oForm2.ShowWindow), oForm2.Desktop
? TRANSFORM(oForm2.MaxHeight), TRANSFORM(oForm2.MaxWidth), TRANSFORM(oForm2.MinHeight), TRANSFORM(oForm2.MinWidth)
? TRANSFORM(oForm2.MaxLeft), TRANSFORM(oForm2.MaxTop), TRANSFORM(oForm2.ScaleMode), TRANSFORM(oForm2.ScrollBars)
? TRANSFORM(oForm2.ViewPortLeft), TRANSFORM(oForm2.ViewPortTop), TRANSFORM(oForm2.ViewPortHeight), TRANSFORM(oForm2.ViewPortWidth)
? TRANSFORM(oForm2.HScrollSmallChange), TRANSFORM(oForm2.VScrollSmallChange), oForm2.ContinuousScroll
? TRANSFORM(oForm2.DataSession), TRANSFORM(oForm2.BufferMode), "[" + oForm2.DEClass + "]", "[" + oForm2.DEClassLibrary + "]"
? oForm2.BindControls, oForm2.ClipControls, oForm2.KeyPreview, oForm2.LockScreen, oForm2.MDIForm
? TRANSFORM(oForm2.DrawMode), TRANSFORM(oForm2.DrawStyle), TRANSFORM(oForm2.DrawWidth), TRANSFORM(oForm2.FillStyle)
? TRANSFORM(oForm2.CurrentX), TRANSFORM(oForm2.CurrentY), TRANSFORM(oForm2.FillColor)
? TRANSFORM(oForm2.Dockable), oForm2.Docked, TRANSFORM(oForm2.DockPosition), TRANSFORM(oForm2.ControlCount)
* nothing on it has the focus and nothing is on it, so the control with the focus is asked after
* by name rather than read: reading it on a form with no controls raises
? PEMSTATUS(oForm2, "ActiveControl", 5), PEMSTATUS(oForm2, "ActiveForm", 5)
? oForm2.ShowInTaskBar, oForm2.ShowTips, oForm2.HalfHeightCaption, oForm2.SizeBox, oForm2.ZoomBox
? oForm2.WhatsThisButton, oForm2.WhatsThisHelp, TRANSFORM(oForm2.ReleaseType), oForm2.AllowOutput
? "[" + oForm2.Icon + "]", TRANSFORM(oForm2.DefOLELCID), TRANSFORM(oForm2.MacDesktop), "[" + oForm2.ClassLibrary + "]"
? TRANSFORM(oForm2.FontCharSet), oForm2.FontOutline, oForm2.FontShadow, oForm2.RightToLeft
? TRANSFORM(oForm2.OLEDragMode), "[" + oForm2.OLEDragPicture + "]", TRANSFORM(oForm2.OLEDropHasData), "[" + oForm2.MouseIcon + "]"

oForm2.Caption = "Ready"
oForm2.Width = 400
oForm2.WindowState = 2
? oForm2.Caption, TRANSFORM(oForm2.Width), TRANSFORM(oForm2.WindowState)

* a form is a window, so it activates, paints and unloads; it draws on itself and docks
? PEMSTATUS(oForm2, "Activate", 5), PEMSTATUS(oForm2, "Deactivate", 5), PEMSTATUS(oForm2, "Paint", 5)
? PEMSTATUS(oForm2, "Load", 5), PEMSTATUS(oForm2, "Unload", 5), PEMSTATUS(oForm2, "QueryUnload", 5)
? PEMSTATUS(oForm2, "Resize", 5), PEMSTATUS(oForm2, "Moved", 5), PEMSTATUS(oForm2, "Scrolled", 5)
? PEMSTATUS(oForm2, "AfterDock", 5), PEMSTATUS(oForm2, "BeforeDock", 5), PEMSTATUS(oForm2, "UnDock", 5)
? PEMSTATUS(oForm2, "Box", 5), PEMSTATUS(oForm2, "Circle", 5), PEMSTATUS(oForm2, "Cls", 5), PEMSTATUS(oForm2, "Line", 5)
? PEMSTATUS(oForm2, "Point", 5), PEMSTATUS(oForm2, "PSet", 5), PEMSTATUS(oForm2, "Print", 5), PEMSTATUS(oForm2, "Draw", 5)
? PEMSTATUS(oForm2, "TextHeight", 5), PEMSTATUS(oForm2, "TextWidth", 5), PEMSTATUS(oForm2, "SetViewPort", 5)
? PEMSTATUS(oForm2, "Dock", 5), PEMSTATUS(oForm2, "GetDockState", 5), PEMSTATUS(oForm2, "WhatsThisMode", 5)
? PEMSTATUS(oForm2, "AddObject", 5), PEMSTATUS(oForm2, "SetAll", 5), PEMSTATUS(oForm2, "SaveAs", 5)
* a form is not a control: it has no anchor, no drag of its own and nothing to enable
? PEMSTATUS(oForm2, "Anchor", 5), PEMSTATUS(oForm2, "DragMode", 5), PEMSTATUS(oForm2, "UIEnable", 5)
? PEMSTATUS(oForm2, "MouseEnter", 5), PEMSTATUS(oForm2, "Sizable", 5), PEMSTATUS(oForm2, "AutoRelease", 5)
* COVERS: Form, TitleBar, Closable, Movable, ControlBox, MaxButton, MinButton, AutoCenter,
* COVERS: AlwaysOnTop, AlwaysOnBottom, WindowState, WindowType, ShowWindow, Desktop, MaxHeight,
* COVERS: MaxWidth, MinHeight, MinWidth, MaxLeft, MaxTop, ScaleMode, ViewPortLeft, ViewPortTop,
* COVERS: ViewPortHeight, ViewPortWidth, HScrollSmallChange, VScrollSmallChange,
* COVERS: ContinuousScroll, DataSession, BufferMode, DEClass, DEClassLibrary, BindControls,
* COVERS: ClipControls, KeyPreview, LockScreen, MDIForm, DrawMode, DrawStyle, DrawWidth,
* COVERS: FillStyle, FillColor, Dockable, Docked, DockPosition, ShowInTaskBar, ShowTips,
* COVERS: HalfHeightCaption, SizeBox, ZoomBox, WhatsThisButton, WhatsThisHelp, ReleaseType,
* COVERS: AllowOutput, Activate, Deactivate, Paint, Load, Unload, QueryUnload, Resize, Moved,
* COVERS: Scrolled, AfterDock, BeforeDock, UnDock, Box, Circle, Cls, Point, PSet, Print, Draw,
* COVERS: TextHeight, TextWidth, SetViewPort, Dock, GetDockState, WhatsThisMode, AddObject,
* COVERS: SaveAs, AutoRelease, Sizable, Icon, DefOLELCID, MacDesktop, ClassLibrary, FontCharSet,
* COVERS: FontOutline, FontShadow, RightToLeft, OLEDragMode, OLEDragPicture, OLEDropHasData,
* COVERS: MouseIcon, ActiveControl, ControlCount
