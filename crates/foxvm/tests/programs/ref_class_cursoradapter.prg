* The CursorAdapter base class: a cursor filled from somewhere that is not a Visual FoxPro table,
* and the pair of events around every thing it does to one.
LOCAL oAdapter
oAdapter = CREATEOBJECT("CursorAdapter")
? oAdapter.BaseClass, oAdapter.Class, oAdapter.Name, "[" + oAdapter.ParentClass + "]"
? "[" + oAdapter.Alias + "]", "[" + oAdapter.DataSourceType + "]", "[" + oAdapter.SelectCmd + "]"
? oAdapter.AllowInsert, oAdapter.AllowUpdate, oAdapter.AllowDelete, oAdapter.SendUpdates
? TRANSFORM(oAdapter.BufferModeOverride), TRANSFORM(oAdapter.WhereType), TRANSFORM(oAdapter.UpdateType)
? TRANSFORM(oAdapter.FetchSize), TRANSFORM(oAdapter.MaxRecords), TRANSFORM(oAdapter.BatchUpdateCount)
? oAdapter.FetchAsNeeded, oAdapter.FetchMemo, oAdapter.CompareMemo, TRANSFORM(oAdapter.UseMemoSize)
? oAdapter.UseTransactions, oAdapter.BreakOnError, oAdapter.NoData, oAdapter.Prepared
? oAdapter.UseCursorSchema, oAdapter.UseDeDataSource, oAdapter.MapBinary, oAdapter.MapVarchar
? TRANSFORM(oAdapter.ADOCodePage), TRANSFORM(oAdapter.CursorStatus), TRANSFORM(oAdapter.Flags)
? TRANSFORM(oAdapter.ConflictCheckType), oAdapter.RefreshTimeStamp, oAdapter.AllowSimultaneousFetch
? "[" + oAdapter.KeyFieldList + "]", "[" + oAdapter.UpdatableFieldList + "]", "[" + oAdapter.UpdateNameList + "]"
? "[" + oAdapter.InsertCmd + "]", "[" + oAdapter.UpdateCmd + "]", "[" + oAdapter.DeleteCmd + "]"
? "[" + oAdapter.RefreshCmd + "]", "[" + oAdapter.CursorSchema + "]", "[" + oAdapter.Tables + "]"
* each of the five may be sent through a connection of its own rather than the adapter's, and
* holds nothing until one is given to it
? VARTYPE(oAdapter.InsertCmdDataSource), VARTYPE(oAdapter.UpdateCmdDataSource)
? VARTYPE(oAdapter.DeleteCmdDataSource), VARTYPE(oAdapter.RefreshCmdDataSource)
? VARTYPE(oAdapter.FetchMemoDataSource), VARTYPE(oAdapter.DataSource)
* the command each of the four has, where it comes from and what it says afterwards
? "[" + oAdapter.InsertCmdDataSourceType + "]", "[" + oAdapter.UpdateCmdDataSourceType + "]"
? "[" + oAdapter.DeleteCmdDataSourceType + "]", "[" + oAdapter.RefreshCmdDataSourceType + "]"
? "[" + oAdapter.FetchMemoDataSourceType + "]", "[" + oAdapter.FetchMemoCmdList + "]"
? "[" + oAdapter.InsertCmdRefreshCmd + "]", "[" + oAdapter.InsertCmdRefreshFieldList + "]"
? "[" + oAdapter.InsertCmdRefreshKeyFieldList + "]", "[" + oAdapter.UpdateCmdRefreshCmd + "]"
? "[" + oAdapter.UpdateCmdRefreshFieldList + "]", "[" + oAdapter.UpdateCmdRefreshKeyFieldList + "]"
? "[" + oAdapter.ConflictCheckCmd + "]", "[" + oAdapter.ConversionFunc + "]", "[" + oAdapter.RefreshAlias + "]"
? "[" + oAdapter.RefreshIgnoreFieldList + "]", "[" + oAdapter.TimestampFieldList + "]"
? "[" + oAdapter.UpdateGram + "]", "[" + oAdapter.UpdateGramSchemaLocation + "]", "[" + oAdapter.ClassLibrary + "]"

oAdapter.Alias = "customer"
oAdapter.SelectCmd = "SELECT * FROM customer"
oAdapter.MaxRecords = 100
? "[" + oAdapter.Alias + "]", "[" + oAdapter.SelectCmd + "]", TRANSFORM(oAdapter.MaxRecords)

* what it does to a cursor, and the before and after of each
? PEMSTATUS(oAdapter, "CursorFill", 5), PEMSTATUS(oAdapter, "CursorAttach", 5), PEMSTATUS(oAdapter, "CursorDetach", 5)
? PEMSTATUS(oAdapter, "CursorRefresh", 5), PEMSTATUS(oAdapter, "RecordRefresh", 5), PEMSTATUS(oAdapter, "AutoOpen", 5)
? PEMSTATUS(oAdapter, "BeforeCursorFill", 5), PEMSTATUS(oAdapter, "AfterCursorFill", 5)
? PEMSTATUS(oAdapter, "BeforeCursorAttach", 5), PEMSTATUS(oAdapter, "AfterCursorAttach", 5)
? PEMSTATUS(oAdapter, "BeforeCursorDetach", 5), PEMSTATUS(oAdapter, "AfterCursorDetach", 5)
? PEMSTATUS(oAdapter, "BeforeCursorClose", 5), PEMSTATUS(oAdapter, "AfterCursorClose", 5)
? PEMSTATUS(oAdapter, "BeforeCursorRefresh", 5), PEMSTATUS(oAdapter, "AfterCursorRefresh", 5)
? PEMSTATUS(oAdapter, "BeforeCursorUpdate", 5), PEMSTATUS(oAdapter, "AfterCursorUpdate", 5)
? PEMSTATUS(oAdapter, "BeforeInsert", 5), PEMSTATUS(oAdapter, "AfterInsert", 5)
? PEMSTATUS(oAdapter, "BeforeUpdate", 5), PEMSTATUS(oAdapter, "AfterUpdate", 5)
? PEMSTATUS(oAdapter, "BeforeDelete", 5), PEMSTATUS(oAdapter, "AfterDelete", 5)
? PEMSTATUS(oAdapter, "BeforeRecordRefresh", 5), PEMSTATUS(oAdapter, "AfterRecordRefresh", 5)
* the four every object has, and one an adapter has not
? PEMSTATUS(oAdapter, "Init", 5), PEMSTATUS(oAdapter, "Destroy", 5), PEMSTATUS(oAdapter, "Error", 5)
? PEMSTATUS(oAdapter, "ReadExpression", 5), PEMSTATUS(oAdapter, "WriteExpression", 5)
? PEMSTATUS(oAdapter, "ReadMethod", 5), PEMSTATUS(oAdapter, "WriteMethod", 5), PEMSTATUS(oAdapter, "SaveAsClass", 5)
? PEMSTATUS(oAdapter, "Click", 5), PEMSTATUS(oAdapter, "Refresh", 5), PEMSTATUS(oAdapter, "Visible", 5)
* COVERS: CursorAdapter, ADOCodePage, AllowDelete, AllowInsert, AllowUpdate,
* COVERS: AllowSimultaneousFetch, BatchUpdateCount, BreakOnError, CompareMemo, ConflictCheckType,
* COVERS: CursorSchema, CursorStatus, DeleteCmd, FetchAsNeeded, FetchMemo, FetchSize, Flags,
* COVERS: InsertCmd, KeyFieldList, MapBinary, MapVarchar, MaxRecords, NoData, Prepared,
* COVERS: RefreshCmd, RefreshTimeStamp, SelectCmd, SendUpdates, Tables, UpdatableFieldList,
* COVERS: UpdateCmd, UpdateNameList, UpdateType, UseCursorSchema, UseDeDataSource, UseMemoSize,
* COVERS: UseTransactions, WhereType, CursorFill, CursorAttach, CursorDetach, CursorRefresh,
* COVERS: RecordRefresh, AutoOpen, BeforeCursorFill, AfterCursorFill, BeforeCursorAttach,
* COVERS: AfterCursorAttach, BeforeCursorDetach, AfterCursorDetach, BeforeCursorClose,
* COVERS: AfterCursorClose, BeforeCursorRefresh, AfterCursorRefresh, BeforeCursorUpdate,
* COVERS: AfterCursorUpdate, BeforeInsert, AfterInsert, BeforeUpdate, AfterUpdate, BeforeDelete,
* COVERS: AfterDelete, BeforeRecordRefresh, AfterRecordRefresh, Init, Destroy, Error,
* COVERS: ReadExpression, WriteExpression, ReadMethod, WriteMethod, SaveAsClass, Click,
* COVERS: InsertCmdDataSource, UpdateCmdDataSource, DeleteCmdDataSource, RefreshCmdDataSource,
* COVERS: FetchMemoDataSource,
* COVERS: InsertCmdDataSourceType, UpdateCmdDataSourceType, DeleteCmdDataSourceType,
* COVERS: RefreshCmdDataSourceType, FetchMemoDataSourceType, FetchMemoCmdList,
* COVERS: InsertCmdRefreshCmd, InsertCmdRefreshFieldList, InsertCmdRefreshKeyFieldList,
* COVERS: UpdateCmdRefreshCmd, UpdateCmdRefreshFieldList, UpdateCmdRefreshKeyFieldList,
* COVERS: ConflictCheckCmd, ConversionFunc, RefreshAlias, RefreshIgnoreFieldList,
* COVERS: TimestampFieldList, UpdateGram, UpdateGramSchemaLocation, ClassLibrary
