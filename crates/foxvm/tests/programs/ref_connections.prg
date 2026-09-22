* The connections a database keeps, and what it is told when they change.
DIMENSION aWhat(1), aAgain(1), aLeft(1)

TEXT TO cProcs NOSHOW
PROCEDURE dbc_BeforeCreateConnection
* this one is handed nothing at all, where every other Before event is handed a name
? "about to add one"
RETURN .T.

PROCEDURE dbc_AfterCreateConnection
LPARAMETERS cName, cSource, cUser, cPassword, cConnect
? "added " + ALLTRIM(cName) + " as " + ALLTRIM(cConnect)

PROCEDURE dbc_BeforeRenameConnection
LPARAMETERS cName, cTo
? "renaming " + ALLTRIM(cName) + " to " + ALLTRIM(cTo)
RETURN .T.

PROCEDURE dbc_AfterRenameConnection
LPARAMETERS cName, cTo
? "renamed"

PROCEDURE dbc_BeforeModifyConnection
LPARAMETERS cName
? "not changing " + ALLTRIM(cName)
RETURN .F.

PROCEDURE dbc_AfterModifyConnection
LPARAMETERS cName
? "changed " + ALLTRIM(cName)

PROCEDURE dbc_BeforeDeleteConnection
LPARAMETERS cName
? "deleting " + ALLTRIM(cName)
RETURN .T.

PROCEDURE dbc_AfterDeleteConnection
LPARAMETERS cName
? "deleted"

PROCEDURE dbc_BeforeModifyTable
LPARAMETERS cName
? "designing table " + ALLTRIM(cName)
RETURN .T.

PROCEDURE dbc_AfterModifyTable
LPARAMETERS cName
? "designed table"

PROCEDURE dbc_BeforeModifyView
LPARAMETERS cName
? "designing view " + ALLTRIM(cName)
RETURN .T.

PROCEDURE dbc_AfterModifyView
LPARAMETERS cName
? "designed view"

PROCEDURE dbc_BeforeModifyProc
? "editing the procedures"
RETURN .T.

PROCEDURE dbc_AfterModifyProc
? "edited the procedures"
ENDTEXT
= STRTOFILE(cProcs, "dbcprocs.prg")

CREATE DATABASE sales
APPEND PROCEDURES FROM dbcprocs.prg
* until the container has been told to, it calls none of its dbc procedures at all - measured
= DBSETPROP("sales", "Database", "DBCEvents", .T.)
CREATE CONNECTION books CONNSTRING "DSN=Books;UID=reader"
CREATE CONNECTION ledger CONNSTRING "DSN=Ledger"
? ADBOBJECTS(aWhat, "CONNECTION"), aWhat(1), aWhat(2)

RENAME CONNECTION ledger TO accounts
? ADBOBJECTS(aAgain, "CONNECTION"), aAgain(2)

* the Before procedure said no, so nothing was changed and nothing followed it
MODIFY CONNECTION accounts
DELETE CONNECTION books
? ADBOBJECTS(aLeft, "CONNECTION"), aLeft(1)

* the designers a database is told about
CREATE TABLE titles (code C(4), name C(20))
MODIFY STRUCTURE
USE
MODIFY VIEW anything
MODIFY PROCEDURE

CLOSE DATABASES
DELETE DATABASE sales.dbc DELETETABLES

* COVERS: CREATE CONNECTION, DELETE CONNECTION, MODIFY CONNECTION, MODIFY STRUCTURE,
* COVERS: RENAME CONNECTION, dbc_AfterCreateConnection, dbc_AfterDeleteConnection,
* COVERS: dbc_AfterModifyConnection, dbc_AfterModifyProc, dbc_AfterModifyTable,
* COVERS: dbc_AfterModifyView, dbc_AfterRenameConnection, dbc_BeforeCreateConnection,
* COVERS: dbc_BeforeDeleteConnection, dbc_BeforeModifyConnection, dbc_BeforeModifyProc,
* COVERS: dbc_BeforeModifyTable, dbc_BeforeModifyView, dbc_BeforeRenameConnection
