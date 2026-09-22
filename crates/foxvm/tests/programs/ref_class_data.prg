* The objects that describe where a form's data comes from, and the one an error arrives in.
LOCAL oEnv, oCursor, oRel, oErr
oEnv = CREATEOBJECT("DataEnvironment")
? oEnv.BaseClass, oEnv.Class, oEnv.Name, "[" + oEnv.ParentClass + "]"
? oEnv.AutoOpenTables, oEnv.AutoCloseTables, TRANSFORM(oEnv.OpenViews)
? "[" + oEnv.InitialSelectedAlias + "]", VARTYPE(oEnv.DataSource), "[" + oEnv.DataSourceType + "]"
? PEMSTATUS(oEnv, "OpenTables", 5), PEMSTATUS(oEnv, "CloseTables", 5), PEMSTATUS(oEnv, "AddObject", 5)
? PEMSTATUS(oEnv, "BeforeOpenTables", 5), PEMSTATUS(oEnv, "AfterCloseTables", 5), PEMSTATUS(oEnv, "Init", 5)

* a cursor says which table, in which order, and how it is buffered
oCursor = CREATEOBJECT("Cursor")
? oCursor.BaseClass, oCursor.Name, "[" + oCursor.Alias + "]", "[" + oCursor.CursorSource + "]"
? "[" + oCursor.Database + "]", "[" + oCursor.Filter + "]", "[" + oCursor.Order + "]"
? TRANSFORM(oCursor.BufferModeOverride), TRANSFORM(oCursor.OrderDirection), oCursor.Exclusive
? oCursor.ReadOnly, oCursor.NoDataOnLoad

* a relation joins two of them
oRel = CREATEOBJECT("Relation")
? oRel.BaseClass, oRel.Name, "[" + oRel.ParentAlias + "]", "[" + oRel.ChildAlias + "]"
? "[" + oRel.ChildOrder + "]", "[" + oRel.RelationalExpr + "]", oRel.OneToMany

* an exception is what a CATCH is handed, and it starts out empty
oErr = CREATEOBJECT("Exception")
? oErr.BaseClass, oErr.Class, oErr.Name, TRANSFORM(oErr.ErrorNo), TRANSFORM(oErr.LineNo)
? "[" + oErr.Message + "]", "[" + oErr.Procedure + "]", "[" + oErr.Details + "]"
? "[" + oErr.LineContents + "]", TRANSFORM(oErr.StackLevel), "[" + oErr.UserValue + "]"
oErr.UserValue = "mine"
? "[" + oErr.UserValue + "]"

* the one an error really did arrive in says what went wrong. The message itself is only asked
* to be there: ours does not end in a full stop where the product's does, which is a difference
* in every message the runtime raises and not something about this object.
TRY
  oCursor.Alias = m.nosuchthing
CATCH TO oErr
  ? TRANSFORM(oErr.ErrorNo), oErr.BaseClass, TRANSFORM(oErr.LineNo), LEN(oErr.Message) > 0
ENDTRY
* COVERS: DataEnvironment, Cursor, Relation, Exception, AutoOpenTables, AutoCloseTables,
* COVERS: OpenViews, InitialSelectedAlias, DataSource, DataSourceType, CloseTables,
* COVERS: BeforeOpenTables, AfterCloseTables, Alias, CursorSource, Database, Filter, Order,
* COVERS: BufferModeOverride, OrderDirection, Exclusive, NoDataOnLoad, ParentAlias, ChildAlias,
* COVERS: OneToMany, ErrorNo, LineNo, Details, LineContents, StackLevel, UserValue, Procedure
