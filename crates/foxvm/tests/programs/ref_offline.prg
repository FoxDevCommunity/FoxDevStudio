* A view taken away from its source, the cursor a server hands back, and a view run again.
*
* The container is told when a view goes offline and comes back, so it carries the procedures
* that say so. dbc_AfterDropOffline is not among them: the product only fires it when the view
* really did come back online, and a view over native data answers .F. every time - so the
* event cannot be made to fire here at all, and this program does not claim it.
LOCAL cSrc
cSrc = "PROCEDURE dbc_BeforeCreateOffline|LPARAMETERS cView, cTable|? 'taking ' + ALLTRIM(cView) + ' offline into ' + LOWER(JUSTFNAME(cTable))|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterCreateOffline|LPARAMETERS cView, cTable|? 'taken offline'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDropOffline|LPARAMETERS cView|? 'putting ' + ALLTRIM(cView) + ' back online'|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterDropOffline|LPARAMETERS cView|? 'this never runs'|"
= STRTOFILE(STRTRAN(cSrc, "|", CHR(13) + CHR(10)), "offlineprocs.prg")

CREATE DATABASE shop
APPEND PROCEDURES FROM offlineprocs.prg
= DBSETPROP("shop", "Database", "DBCEvents", .T.)
CREATE TABLE stock (code C(4), name C(12))
INSERT INTO stock VALUES ("A1", "Bolt")
INSERT INTO stock VALUES ("B2", "Nut")
USE
* no ADD TABLE: a table made while a database is open is in it already, and the product refuses
* to add it a second time - see ref_addtable.prg
CREATE SQL VIEW cheap AS SELECT * FROM stock WHERE code = "A1"

USE cheap
? ALIAS(), RECCOUNT()
? ISTRANSACTABLE("cheap")
? MAKETRANSACTABLE("cheap"), ISTRANSACTABLE("cheap")
? CREATEOFFLINE("cheap")
* one call to a line: a procedure that prints while an expression is being worked out puts its
* text in the middle of the line the expression is on
? DROPOFFLINE("cheap")
? DROPOFFLINE("cheap")

* which cursor a server hands back, as the work area it is in - which CREATE SQL VIEW may not
* leave at the one a fresh USE would pick, since a view keeps its own source open behind it
? GETRESULTSET() == 0
? SETRESULTSET("cheap") == 0, GETRESULTSET() == SELECT("cheap")
? CLEARRESULTSET() == SELECT("cheap"), GETRESULTSET() == 0
* naming a cursor nothing answers to refuses rather than leaving the marker where it was
LOCAL oErr, cSaid
TRY
  SETRESULTSET("nosuchcursor")
  cSaid = "no error"
CATCH TO oErr
  cSaid = "error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid
* a cursor this runtime makes is never filled by a CursorAdapter object, so asking for one is
* refused rather than answered .NULL.
TRY
  GETCURSORADAPTER("cheap")
  cSaid = "no error"
CATCH TO oErr
  cSaid = "error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid

* the view read again from what the tables say now
? REQUERY("cheap"), RECCOUNT()
USE

CLOSE DATABASES
DELETE DATABASE shop.dbc DELETETABLES

* COVERS: CLEARRESULTSET, CREATEOFFLINE, DROPOFFLINE, GETCURSORADAPTER, GETRESULTSET,
* COVERS: ISTRANSACTABLE, MAKETRANSACTABLE, REQUERY, SETRESULTSET, dbc_AfterCreateOffline,
* COVERS: dbc_BeforeCreateOffline, dbc_BeforeDropOffline
