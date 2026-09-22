* A database and the things listed in it: its tables, its views, its stored procedures and the
* triggers on a table. Nothing here prints a path, because the folder a program runs in is a
* different one every time it is measured.
SET SAFETY OFF
DIMENSION aWhat[1], aViews[1]
CREATE DATABASE yard
CREATE TABLE tools (code C(4), name C(10))
INSERT INTO tools (code, name) VALUES ("SAW", "Handsaw")
CREATE SQL VIEW vtools AS SELECT * FROM tools
? DBUSED("yard"), TRANSFORM(ADBOBJECTS(aWhat, "TABLE"))
? TRANSFORM(ADBOBJECTS(aViews, "VIEW")), aViews(1)

* a view renamed and then taken out of the database
RENAME VIEW vtools TO vkit
? TRANSFORM(ADBOBJECTS(aViews, "VIEW")), aViews(1)
DELETE VIEW vkit
? TRANSFORM(ADBOBJECTS(aViews, "VIEW"))

* the procedures a database stores, written out and read back in
? TRANSFORM(STRTOFILE("PROCEDURE Greet" + CHR(13) + CHR(10) + "RETURN 'hello'" + CHR(13) + CHR(10) + "ENDPROC" + CHR(13) + CHR(10), "procs.txt"))
APPEND PROCEDURES FROM procs.txt
COPY PROCEDURES TO back.txt
? FILE("back.txt")

* the triggers on a table, taken off again
DELETE TRIGGER ON tools FOR INSERT
DELETE TRIGGER ON tools FOR UPDATE
DELETE TRIGGER ON tools FOR DELETE
? TRANSFORM(ADBOBJECTS(aWhat, "TABLE"))

* what is crossed off a database goes when it is packed, which needs it to itself
CLOSE TABLES ALL
CLOSE DATABASES ALL
OPEN DATABASE yard EXCLUSIVE
PACK DATABASE
? TRANSFORM(ADBOBJECTS(aWhat, "TABLE"))
CLOSE DATABASES ALL
? DBUSED("yard")

* a table whose database has gone still says it belongs to one; FREE TABLE is what unties it
ERASE yard.dbc
ERASE yard.dct
ERASE yard.dcx
FREE TABLE tools
USE tools
? TRANSFORM(RECCOUNT()), ALLTRIM(name)
USE
ERASE procs.txt
ERASE back.txt
* COVERS: APPEND PROCEDURES, COPY PROCEDURES, DELETE TRIGGER, DELETE VIEW, FREE TABLE,
* COVERS: PACK DATABASE, RENAME VIEW
