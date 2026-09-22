* Wave 6 of the language reference: the database container.
? DBC() == "", DBUSED("depot")
CREATE DATABASE depot
? DBC(), DBUSED("depot")
CREATE TABLE parts (code C(6), name C(12), price N(8,2))
INSERT INTO parts (code, name, price) VALUES ("A1", "Bolt", 2.50)
INSERT INTO parts (code, name, price) VALUES ("B2", "Nut", 1.75)
INSERT INTO parts (code, name, price) VALUES ("C3", "Bearing", 40.00)
USE
* a table made while a database is open is in it already; adding it again is error 1537, which
* ref_addtable.prg measures
? INDBC("parts", "TABLE"), INDBC("nothing", "TABLE")
LOCAL aTables(1)
? ADBOBJECTS(aTables, "TABLE"), aTables(1)
* what the container remembers about what it holds
? DBSETPROP("parts", "TABLE", "Comment", "the parts we stock")
? DBGETPROP("parts", "TABLE", "Comment")
? DBGETPROP("depot", "DATABASE", "Comment") == ""
LIST DATABASE
* a view is a SELECT the database keeps
CREATE SQL VIEW dear AS SELECT code, name, price FROM parts WHERE price > 2
? DBGETPROP("dear", "VIEW", "SQL")
LOCAL aViews(1)
? ADBOBJECTS(aViews, "VIEW"), aViews(1)
USE dear
? ALIAS(), RECCOUNT()
GO TOP
? ALLTRIM(name)
GO BOTTOM
? ALLTRIM(name)
USE
* the table can be renamed in the container, and taken out of it
RENAME TABLE parts TO components
LIST DATABASE
REMOVE TABLE components
? INDBC("components", "TABLE")
DROP VIEW dear
LIST DATABASE
VALIDATE DATABASE
* and the database itself is opened again from the file it was written to
SET DATABASE TO
? DBC() == ""
SET DATABASE TO depot
? DBC() == "depot.dbc"
CLOSE DATABASES
? DBC() == "", DBUSED("depot")
OPEN DATABASE depot
? DBUSED("depot")
LIST DATABASE
DELETE DATABASE depot
? DBC() == "", FILE("depot.dbc")
* COVERS: ADBOBJECTS, ALIAS, CLOSE, CREATE DATABASE, CREATE SQL VIEW,
* COVERS: CREATE TABLE, DBC, DBGETPROP, DBSETPROP, DBUSED, DELETE DATABASE, DROP VIEW, FILE,
* COVERS: INDBC, INSERT, LIST DATABASE, OPEN DATABASE, RECCOUNT, REMOVE TABLE, RENAME TABLE,
* COVERS: SET DATABASE, USE, VALIDATE DATABASE
