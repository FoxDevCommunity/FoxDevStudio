* The Grid and the two classes it is built out of: Column and Header.
LOCAL oGrid, oCol, oHead
oGrid = CREATEOBJECT("Grid")
? oGrid.BaseClass, oGrid.Class, oGrid.Name, "[" + oGrid.ParentClass + "]"
? TRANSFORM(oGrid.Width), TRANSFORM(oGrid.Height), TRANSFORM(oGrid.ColumnCount), TRANSFORM(oGrid.RowHeight)
? TRANSFORM(oGrid.HeaderHeight), TRANSFORM(oGrid.GridLines), TRANSFORM(oGrid.GridLineColor), TRANSFORM(oGrid.GridLineWidth)
? oGrid.DeleteMark, oGrid.RecordMark, oGrid.SplitBar, TRANSFORM(oGrid.Partition), TRANSFORM(oGrid.Panel)
? oGrid.PanelLink, TRANSFORM(oGrid.LeftColumn), TRANSFORM(oGrid.LockColumns), TRANSFORM(oGrid.LockColumnsLeft)
? oGrid.AllowAddNew, oGrid.AllowCellSelection, oGrid.AllowHeaderSizing, oGrid.AllowRowSizing
? TRANSFORM(oGrid.AllowAutoColumnFit), oGrid.Highlight, oGrid.HighlightRow, TRANSFORM(oGrid.HighlightStyle)
? TRANSFORM(oGrid.HighlightBackColor), TRANSFORM(oGrid.HighlightForeColor), TRANSFORM(oGrid.HighlightRowLineWidth)
? "[" + oGrid.RecordSource + "]", TRANSFORM(oGrid.RecordSourceType), "[" + oGrid.ChildOrder + "]", "[" + oGrid.LinkMaster + "]"
? "[" + oGrid.RelationalExpr + "]", oGrid.Optimize, oGrid.ReadOnly, TRANSFORM(oGrid.ScrollBars)
? TRANSFORM(oGrid.ActiveColumn), TRANSFORM(oGrid.ActiveRow), TRANSFORM(oGrid.RelativeColumn), TRANSFORM(oGrid.RelativeRow)
? TRANSFORM(oGrid.RowColChange), TRANSFORM(oGrid.View), TRANSFORM(oGrid.Value)

? PEMSTATUS(oGrid, "AddColumn", 5), PEMSTATUS(oGrid, "DeleteColumn", 5), PEMSTATUS(oGrid, "AutoFit", 5)
? PEMSTATUS(oGrid, "ActivateCell", 5), PEMSTATUS(oGrid, "DoScroll", 5), PEMSTATUS(oGrid, "GridHitTest", 5)
? PEMSTATUS(oGrid, "AfterRowColChange", 5), PEMSTATUS(oGrid, "BeforeRowColChange", 5), PEMSTATUS(oGrid, "Deleted", 5)
? PEMSTATUS(oGrid, "Scrolled", 5), PEMSTATUS(oGrid, "Columns", 5), PEMSTATUS(oGrid, "Caption", 5)

* a column holds one field, and the expressions that change how each row of it looks
oCol = CREATEOBJECT("Column")
? oCol.BaseClass, oCol.Name, TRANSFORM(oCol.Width), TRANSFORM(oCol.ColumnOrder), TRANSFORM(oCol.Alignment)
? oCol.Bound, oCol.Sparse, oCol.Movable, oCol.Resizable, oCol.SelectOnEntry, oCol.ReadOnly
? "[" + oCol.ControlSource + "]", "[" + oCol.CurrentControl + "]", "[" + oCol.HeaderClass + "]"
? "[" + oCol.DynamicBackColor + "]", "[" + oCol.DynamicForeColor + "]", "[" + oCol.DynamicInputMask + "]"
? "[" + oCol.DynamicAlignment + "]", "[" + oCol.DynamicFontName + "]", "[" + oCol.DynamicCurrentControl + "]"
? "[" + oCol.DynamicFontSize + "]", "[" + oCol.DynamicFontOutline + "]", "[" + oCol.DynamicFontShadow + "]"
? "[" + oCol.HeaderClassLibrary + "]", TRANSFORM(oCol.FontCharSet), oCol.FontOutline, oCol.FontShadow
? PEMSTATUS(oCol, "AddObject", 5), PEMSTATUS(oCol, "AutoFit", 5), PEMSTATUS(oCol, "Click", 5)

* a header is the strip at the top of a column
oHead = CREATEOBJECT("Header")
? oHead.BaseClass, oHead.Name, oHead.Caption, TRANSFORM(oHead.Alignment), oHead.WordWrap
? TRANSFORM(oHead.BackColor), TRANSFORM(oHead.ForeColor), oHead.FontName, TRANSFORM(oHead.FontSize)
? PEMSTATUS(oHead, "Click", 5), PEMSTATUS(oHead, "DblClick", 5), PEMSTATUS(oHead, "Width", 5)
* COVERS: Grid, Column, Header, RowHeight, HeaderHeight, GridLines, GridLineColor, GridLineWidth,
* COVERS: DeleteMark, RecordMark, SplitBar, Partition, Panel, PanelLink, LeftColumn, LockColumns,
* COVERS: LockColumnsLeft, AllowAddNew, AllowCellSelection, AllowHeaderSizing, AllowRowSizing,
* COVERS: AllowAutoColumnFit, Highlight, HighlightRow, HighlightStyle, HighlightBackColor,
* COVERS: HighlightForeColor, HighlightRowLineWidth, RecordSource, RecordSourceType, ChildOrder,
* COVERS: LinkMaster, RelationalExpr, Optimize, ActiveColumn, ActiveRow, RelativeColumn,
* COVERS: RelativeRow, RowColChange, View, ColumnOrder, Bound, Sparse, Movable, Resizable,
* COVERS: CurrentControl, HeaderClass, DynamicInputMask, DynamicAlignment, DynamicFontName,
* COVERS: DynamicCurrentControl, AddColumn, DeleteColumn, AutoFit, ActivateCell, DoScroll,
* COVERS: AfterRowColChange, BeforeRowColChange, Deleted, Columns, DynamicFontSize,
* COVERS: DynamicFontOutline, DynamicFontShadow, HeaderClassLibrary, FontCharSet, FontOutline,
* COVERS: FontShadow
