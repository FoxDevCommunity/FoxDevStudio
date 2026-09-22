* The Toolbar and the two classes beside it that a form is not made of: Separator and Control,
* and the ProjectHook a project runs its own code through.
LOCAL oBar, oGap, oCtl, oHook
oBar = CREATEOBJECT("Toolbar")
? oBar.BaseClass, oBar.Class, oBar.Name, "[" + oBar.ParentClass + "]"
? oBar.Caption, TRANSFORM(oBar.Width), TRANSFORM(oBar.Height), TRANSFORM(oBar.ControlCount)
? oBar.Docked, TRANSFORM(oBar.DockPosition), oBar.Movable, oBar.Sizable, oBar.ControlBox
? TRANSFORM(oBar.BackColor), TRANSFORM(oBar.ForeColor), oBar.ShowTips, TRANSFORM(oBar.ShowWindow)
? TRANSFORM(oBar.DataSession), oBar.KeyPreview, oBar.LockScreen, TRANSFORM(oBar.ScaleMode)
? PEMSTATUS(oBar, "Dock", 5), PEMSTATUS(oBar, "Show", 5), PEMSTATUS(oBar, "Hide", 5), PEMSTATUS(oBar, "Release", 5)
? PEMSTATUS(oBar, "AfterDock", 5), PEMSTATUS(oBar, "BeforeDock", 5), PEMSTATUS(oBar, "UnDock", 5)
? PEMSTATUS(oBar, "Activate", 5), PEMSTATUS(oBar, "Deactivate", 5), PEMSTATUS(oBar, "Paint", 5)
? PEMSTATUS(oBar, "GetDockState", 5), PEMSTATUS(oBar, "Box", 5), PEMSTATUS(oBar, "Anchor", 5)

* a separator is the gap between two buttons on a toolbar, and holds almost nothing
oGap = CREATEOBJECT("Separator")
? oGap.BaseClass, oGap.Class, oGap.Name, TRANSFORM(oGap.Style), oGap.Enabled, oGap.Visible
? PEMSTATUS(oGap, "Width", 5), PEMSTATUS(oGap, "Click", 5), PEMSTATUS(oGap, "ZOrder", 5)

* Control is the class every visual control is built on: a container with no children of its own
oCtl = CREATEOBJECT("Control")
? oCtl.BaseClass, oCtl.Class, oCtl.Name, TRANSFORM(oCtl.Width), TRANSFORM(oCtl.Height)
? TRANSFORM(oCtl.BackColor), TRANSFORM(oCtl.BorderWidth), TRANSFORM(oCtl.SpecialEffect), TRANSFORM(oCtl.ControlCount)
? PEMSTATUS(oCtl, "Draw", 5), PEMSTATUS(oCtl, "SetFocus", 5), PEMSTATUS(oCtl, "AddObject", 5)
? PEMSTATUS(oCtl, "Moved", 5), PEMSTATUS(oCtl, "Resize", 5), PEMSTATUS(oCtl, "UIEnable", 5)

* a project hook is asked before the project does anything, and says whether it may
oHook = CREATEOBJECT("ProjectHook")
? oHook.BaseClass, oHook.Class, oHook.Name, TRANSFORM(oHook.OLEDropMode), TRANSFORM(oHook.OLEDropEffects)
? PEMSTATUS(oHook, "QueryAddFile", 5), PEMSTATUS(oHook, "QueryModifyFile", 5), PEMSTATUS(oHook, "QueryNewFile", 5)
? PEMSTATUS(oHook, "QueryRemoveFile", 5), PEMSTATUS(oHook, "QueryRunFile", 5), PEMSTATUS(oHook, "AfterBuild", 5)
? PEMSTATUS(oHook, "BeforeBuild", 5), PEMSTATUS(oHook, "SCCInit", 5), PEMSTATUS(oHook, "SCCDestroy", 5)
? PEMSTATUS(oHook, "Activate", 5), PEMSTATUS(oHook, "Deactivate", 5), PEMSTATUS(oHook, "OLEDragDrop", 5)
? PEMSTATUS(oHook, "OLEDragOver", 5), PEMSTATUS(oHook, "OLEGiveFeedback", 5), PEMSTATUS(oHook, "OLEStartDrag", 5)
* COVERS: ToolBar, Separator, Control, ProjectHook, Sizable, ShowWindow, Show, Hide, Release,
* COVERS: Dock, GetDockState, QueryAddFile, QueryModifyFile, QueryNewFile, QueryRemoveFile,
* COVERS: QueryRunFile, AfterBuild, SCCInit, SCCDestroy, OLEDragOver, OLEGiveFeedback,
* COVERS: OLEStartDrag, OLEDropMode, OLEDropEffects, Draw, Moved
