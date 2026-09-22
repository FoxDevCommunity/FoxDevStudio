* The controls that are drawn rather than used: Image, Line and Shape, and the Timer beside them.
LOCAL oImage, oLine, oShape, oTimer
oImage = CREATEOBJECT("Image")
? oImage.BaseClass, oImage.Class, oImage.Name, "[" + oImage.ParentClass + "]"
? TRANSFORM(oImage.Width), TRANSFORM(oImage.Height), TRANSFORM(oImage.Stretch), TRANSFORM(oImage.RotateFlip)
? "[" + oImage.Picture + "]", "[" + oImage.PictureVal + "]", TRANSFORM(oImage.BackStyle), TRANSFORM(oImage.BorderStyle)
? TRANSFORM(oImage.BorderColor), oImage.Enabled, oImage.Visible, oImage.Themes
? PEMSTATUS(oImage, "Click", 5), PEMSTATUS(oImage, "Refresh", 5), PEMSTATUS(oImage, "SetFocus", 5)

* a line is one-dimensional: it has a slant instead of a shape
oLine = CREATEOBJECT("Line")
? oLine.BaseClass, oLine.Name, TRANSFORM(oLine.Width), TRANSFORM(oLine.Height)
? "[" + oLine.LineSlant + "]", TRANSFORM(oLine.BorderWidth), TRANSFORM(oLine.BorderStyle), TRANSFORM(oLine.DrawMode)
? TRANSFORM(oLine.Rotation), "[" + oLine.PolyPoints + "]"
oLine.LineSlant = "/"
? "[" + oLine.LineSlant + "]"

* a shape is a box that can be given corners
oShape = CREATEOBJECT("Shape")
? oShape.BaseClass, oShape.Name, TRANSFORM(oShape.Curvature), TRANSFORM(oShape.FillStyle), TRANSFORM(oShape.FillColor)
? TRANSFORM(oShape.BackColor), TRANSFORM(oShape.BackStyle), TRANSFORM(oShape.SpecialEffect), TRANSFORM(oShape.BorderWidth)
oShape.Curvature = 99
? TRANSFORM(oShape.Curvature)
? PEMSTATUS(oShape, "Curvature", 5), PEMSTATUS(oLine, "Curvature", 5), PEMSTATUS(oImage, "Curvature", 5)

* a timer is not drawn at all: it has a place and a size, and nothing else of a control
oTimer = CREATEOBJECT("Timer")
? oTimer.BaseClass, oTimer.Class, oTimer.Name, TRANSFORM(oTimer.Interval), oTimer.Enabled
? TRANSFORM(oTimer.Left), TRANSFORM(oTimer.Top), TRANSFORM(oTimer.Width), TRANSFORM(oTimer.Height)
oTimer.Interval = 500
? TRANSFORM(oTimer.Interval)
? PEMSTATUS(oTimer, "Timer", 5), PEMSTATUS(oTimer, "Reset", 5), PEMSTATUS(oTimer, "Init", 5)
? PEMSTATUS(oTimer, "Visible", 5), PEMSTATUS(oTimer, "Click", 5), PEMSTATUS(oTimer, "Anchor", 5)
* COVERS: Image, Line, Shape, Timer, Stretch, RotateFlip, PictureVal, LineSlant, PolyPoints,
* COVERS: Curvature, Interval, Reset, Rotation
