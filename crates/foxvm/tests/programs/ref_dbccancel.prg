* What a database event that answers .F. actually stops.
*
* Most refusals are quiet: the command does nothing and no error is raised. Opening a table,
* opening the container and packing it are the three that raise "File access is denied" - all
* measured, by driving a container whose procedures say no one at a time and looking at what
* was left behind.
LOCAL cSrc, oErr, cSaid
cSrc = "PROCEDURE dbc_BeforeCreateTable|LPARAMETERS cFile, cName|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_AfterCreateTable|LPARAMETERS cFile, cName|? 'the table was made after all'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeRemoveTable|LPARAMETERS cName, lDelete, lRecycle|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_AfterRemoveTable|LPARAMETERS cName, lDelete, lRecycle|? 'it was removed after all'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeOpenTable|LPARAMETERS cName|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_BeforeCloseTable|LPARAMETERS cName|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDBGetProp|LPARAMETERS cName, cType, cProperty|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDBSetProp|LPARAMETERS cName, cType, cProperty, eValue|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_PackData|RETURN .F.|"
cSrc = cSrc + "PROCEDURE dbc_CloseData|LPARAMETERS cDb, lAll|RETURN .F.|"
= STRTOFILE(STRTRAN(cSrc, "|", CHR(13) + CHR(10)), "noprocs.prg")

CREATE DATABASE fort
APPEND PROCEDURES FROM noprocs.prg
CREATE TABLE keep (code C(4))
USE
= DBSETPROP("fort", "Database", "DBCEvents", .T.)

* a refused CREATE TABLE writes no file, takes no work area and says nothing
CREATE TABLE nomore (code C(4))
? FILE("nomore.dbf"), USED("nomore")

* a refused REMOVE TABLE leaves the table where it was
REMOVE TABLE keep
? INDBC("keep", "TABLE")

* a refused DBSETPROP answers .F. and a refused DBGETPROP answers .NULL.
? DBSETPROP("keep", "Table", "Comment", "never")
? ISNULL(DBGETPROP("keep", "Table", "Comment"))

* a refused open is one of the three that raise an error
TRY
  USE keep
  cSaid = "no error"
CATCH TO oErr
  cSaid = "error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid, USED("keep")

* so is a refused pack
TRY
  PACK DATABASE
  cSaid = "no error"
CATCH TO oErr
  cSaid = "error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid

* a refused close leaves the container open
CLOSE DATABASES
? DBUSED("fort")

* COVERS: dbc_BeforeCloseTable, dbc_BeforeCreateTable, dbc_BeforeDBGetProp,
* COVERS: dbc_BeforeDBSetProp, dbc_BeforeOpenTable, dbc_BeforeRemoveTable, dbc_CloseData,
* COVERS: dbc_PackData
