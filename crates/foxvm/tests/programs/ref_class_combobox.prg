* The two controls that hold a list: ComboBox and ListBox.
LOCAL oCombo, oList
oCombo = CREATEOBJECT("ComboBox")
? oCombo.BaseClass, oCombo.Class, oCombo.Name, "[" + oCombo.ParentClass + "]"
? TRANSFORM(oCombo.Width), TRANSFORM(oCombo.Height), TRANSFORM(oCombo.Style), TRANSFORM(oCombo.Alignment)
? "[" + oCombo.RowSource + "]", TRANSFORM(oCombo.RowSourceType), TRANSFORM(oCombo.ColumnCount)
? TRANSFORM(oCombo.BoundColumn), oCombo.BoundTo, "[" + oCombo.ColumnWidths + "]", oCombo.ColumnLines
? TRANSFORM(oCombo.ListCount), TRANSFORM(oCombo.ListIndex), TRANSFORM(oCombo.TopIndex), TRANSFORM(oCombo.TopItemID)
? TRANSFORM(oCombo.NewIndex), TRANSFORM(oCombo.NewItemID), TRANSFORM(oCombo.FirstElement), TRANSFORM(oCombo.NumberOfElements)
? oCombo.Sorted, oCombo.IncrementalSearch, oCombo.ItemTips, TRANSFORM(oCombo.DisplayCount)
? "[" + oCombo.Value + "]", "[" + oCombo.DisplayValue + "]", TRANSFORM(oCombo.SelectedItemBackColor)
? TRANSFORM(oCombo.ItemBackColor), TRANSFORM(oCombo.ItemForeColor), TRANSFORM(oCombo.PictureSelectionDisplay)
? TRANSFORM(oCombo.ItemData), TRANSFORM(oCombo.ItemIDData), "[" + oCombo.ListItem + "]", TRANSFORM(oCombo.ListItemID)
? oCombo.Selected, oCombo.SelectedID, TRANSFORM(oCombo.OLEDropTextInsertion), "[" + oCombo.ClassLibrary + "]"

* what a write reads back
oCombo.RowSourceType = 1
oCombo.RowSource = "Alpha,Beta"
oCombo.Sorted = .T.
? TRANSFORM(oCombo.RowSourceType), oCombo.RowSource, oCombo.Sorted

? PEMSTATUS(oCombo, "DropDown", 5), PEMSTATUS(oCombo, "UpClick", 5), PEMSTATUS(oCombo, "DownClick", 5)
? PEMSTATUS(oCombo, "AddItem", 5), PEMSTATUS(oCombo, "AddListItem", 5), PEMSTATUS(oCombo, "RemoveItem", 5)
? PEMSTATUS(oCombo, "RemoveListItem", 5), PEMSTATUS(oCombo, "Requery", 5), PEMSTATUS(oCombo, "Clear", 5)
? PEMSTATUS(oCombo, "IndexToItemID", 5), PEMSTATUS(oCombo, "ItemIDToIndex", 5)

* a list box shows its items all at once, so it has a mover and can select more than one
oList = CREATEOBJECT("ListBox")
? oList.BaseClass, oList.Name, TRANSFORM(oList.Width), TRANSFORM(oList.Height)
? oList.MultiSelect, oList.MoverBars, oList.IntegralHeight, TRANSFORM(oList.AutoHideScrollBar)
? TRANSFORM(oList.ColumnCount), "[" + oList.RowSource + "]", TRANSFORM(oList.RowSourceType)
? PEMSTATUS(oList, "MoveItem", 5), PEMSTATUS(oList, "OnMoveItem", 5), PEMSTATUS(oList, "DropDown", 5)
* COVERS: ComboBox, ListBox, RowSource, RowSourceType, ColumnCount, BoundColumn, BoundTo,
* COVERS: ColumnWidths, ColumnLines, ListCount, ListIndex, TopIndex, TopItemID, NewIndex,
* COVERS: NewItemID, FirstElement, NumberOfElements, Sorted, IncrementalSearch, ItemTips,
* COVERS: DisplayCount, DisplayValue, SelectedItemBackColor, SelectedItemForeColor,
* COVERS: PictureSelectionDisplay, MultiSelect, MoverBars, AutoHideScrollBar, List, AddItem,
* COVERS: AddListItem, RemoveItem, RemoveListItem, Requery, Clear, IndexToItemID, ItemIDToIndex,
* COVERS: MoveItem, OnMoveItem, DropDown, UpClick, DownClick, ItemData, ItemIDData, ListItem,
* COVERS: ListItemID, Selected, SelectedID, OLEDropTextInsertion, ClassLibrary
