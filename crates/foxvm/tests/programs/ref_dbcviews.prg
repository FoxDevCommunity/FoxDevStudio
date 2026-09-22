* What a database container is told about its views, and about its own designer.
*
* Deleting a view is the odd one out: the product fires nothing at all for DELETE VIEW or DROP
* VIEW, though dbc_BeforeDeleteView and dbc_AfterDeleteView are documented events - measured,
* with both procedures in place and every other event of the run firing.
LOCAL cSrc
cSrc = "PROCEDURE dbc_BeforeCreateView|LPARAMETERS cName|? 'before create view ' + ALLTRIM(cName)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterCreateView|LPARAMETERS cName, lRemote|? 'after create view ' + ALLTRIM(cName) + ' ' + TRANSFORM(lRemote)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeRenameView|LPARAMETERS cFrom, cTo|? 'before rename view ' + ALLTRIM(cFrom) + ' to ' + ALLTRIM(cTo)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterRenameView|LPARAMETERS cFrom, cTo|? 'after rename view ' + ALLTRIM(cTo)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDeleteView|LPARAMETERS cName|? 'this never runs'|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterDeleteView|LPARAMETERS cName|? 'nor does this'|"
cSrc = cSrc + "PROCEDURE dbc_ModifyData|LPARAMETERS cDb, lNoWait, lNoEdit|? 'designing ' + LOWER(JUSTFNAME(cDb)) + ' ' + TRANSFORM(lNoWait) + TRANSFORM(lNoEdit)|RETURN .T.|"
= STRTOFILE(STRTRAN(cSrc, "|", CHR(13) + CHR(10)), "viewprocs.prg")

CREATE DATABASE atlas
APPEND PROCEDURES FROM viewprocs.prg
= DBSETPROP("atlas", "Database", "DBCEvents", .T.)
CREATE TABLE places (code C(4), name C(12))
USE

CREATE SQL VIEW nearby AS SELECT * FROM places
RENAME VIEW nearby TO handy
DELETE VIEW handy
? INDBC("handy", "VIEW")

* The Database Designer, which has to be asked for with NOWAIT or the product waits for a
* person. The container hears it and can keep the window from opening at all.
MODIFY DATABASE NOWAIT NOEDIT

* COVERS: dbc_AfterCreateView, dbc_AfterRenameView, dbc_BeforeCreateView, dbc_BeforeRenameView,
* COVERS: dbc_ModifyData
