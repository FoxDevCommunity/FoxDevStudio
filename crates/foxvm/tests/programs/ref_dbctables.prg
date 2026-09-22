* What a database container is told as its tables are made, opened, moved and taken away.
*
* Every name a dbc procedure is handed arrives lower-cased, and the files are full paths, so
* what is printed here is the file name rather than the folder it happened to sit in. The
* procedures are built as one string with `|` for a line break, because a TEXT block is echoed
* into the capture even with NOSHOW and would drown the output.
LOCAL cSrc
cSrc = "PROCEDURE dbc_BeforeCreateTable|LPARAMETERS cFile, cName|? 'before create ' + ALLTRIM(cName) + ' in ' + LOWER(JUSTFNAME(cFile))|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterCreateTable|LPARAMETERS cFile, cName|? 'after create ' + ALLTRIM(cName)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeOpenTable|LPARAMETERS cName|? 'before open ' + ALLTRIM(cName)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterOpenTable|LPARAMETERS cName|? 'after open ' + ALLTRIM(cName)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeCloseTable|LPARAMETERS cName|? 'before close ' + ALLTRIM(cName)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterCloseTable|LPARAMETERS cName|? 'after close ' + ALLTRIM(cName)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeAddTable|LPARAMETERS cFile, cName|? 'before add ' + ALLTRIM(cName) + ' from ' + LOWER(JUSTFNAME(cFile))|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterAddTable|LPARAMETERS cFile, cName|? 'after add ' + ALLTRIM(cName)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeRemoveTable|LPARAMETERS cName, lDelete, lRecycle|? 'before remove ' + ALLTRIM(cName) + ' ' + TRANSFORM(lDelete) + TRANSFORM(lRecycle)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterRemoveTable|LPARAMETERS cName, lDelete, lRecycle|? 'after remove ' + ALLTRIM(cName)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeRenameTable|LPARAMETERS cFrom, cTo|? 'before rename ' + ALLTRIM(cFrom) + ' to ' + ALLTRIM(cTo)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterRenameTable|LPARAMETERS cFrom, cTo|? 'after rename ' + ALLTRIM(cTo)|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDropTable|LPARAMETERS cName, lRecycle|? 'before drop ' + ALLTRIM(cName) + ' ' + TRANSFORM(lRecycle)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterDropTable|LPARAMETERS cName, lRecycle|? 'after drop ' + ALLTRIM(cName)|"
= STRTOFILE(STRTRAN(cSrc, "|", CHR(13) + CHR(10)), "tableprocs.prg")

CREATE DATABASE shop
APPEND PROCEDURES FROM tableprocs.prg
* a container whose events have not been switched on calls none of its procedures at all
CREATE TABLE quiet (code C(4))
USE
= DBSETPROP("shop", "Database", "DBCEvents", .T.)

CREATE TABLE stock (code C(4), name C(12))
USE
* the Before event hears the table's name, the After event hears the alias it was opened under
USE stock ALIAS spare
USE

* a free table is none of the container's business until it is added to it
CREATE TABLE loose FREE (code C(4))
USE
ADD TABLE loose
REMOVE TABLE loose RECYCLE

CREATE TABLE extra (code C(4))
USE
RENAME TABLE extra TO spares
DROP TABLE stock RECYCLE

CLOSE DATABASES

* COVERS: dbc_AfterAddTable, dbc_AfterCloseTable, dbc_AfterCreateTable, dbc_AfterDropTable,
* COVERS: dbc_AfterOpenTable, dbc_AfterRemoveTable, dbc_AfterRenameTable, dbc_BeforeAddTable,
* COVERS: dbc_BeforeCloseTable, dbc_BeforeCreateTable, dbc_BeforeDropTable,
* COVERS: dbc_BeforeOpenTable, dbc_BeforeRemoveTable, dbc_BeforeRenameTable
