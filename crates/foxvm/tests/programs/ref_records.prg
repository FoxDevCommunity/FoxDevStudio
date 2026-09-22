* Finding a record, marking one, and the commands that make a table out of another table's
* rows. Everything here is what a program does while it runs, so a golden can watch all of it.
SET SAFETY OFF
? SET("SAFETY")
CREATE TABLE crew (name C(10), dept C(6), pay N(8,2))
INSERT INTO crew (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO crew (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
INSERT INTO crew (name, dept, pay) VALUES ("Curtis", "SALES", 2750)
INSERT INTO crew (name, dept, pay) VALUES ("Bell", "ADMIN", 5000)

* LOCATE finds the first record its condition holds for; CONTINUE finds the next one after that
LOCATE FOR dept = "ADMIN"
? FOUND(), TRANSFORM(RECNO()), ALLTRIM(name)
CONTINUE
? FOUND(), TRANSFORM(RECNO()), ALLTRIM(name)
CONTINUE
? FOUND(), EOF(), TRANSFORM(RECNO())
LOCATE FOR pay > 9000
? FOUND(), EOF()

* GOTO is GO under another name: a record number, or where in the table to stand
GOTO 3
? ALLTRIM(name)
GOTO TOP
? ALLTRIM(name)
GOTO BOTTOM
? ALLTRIM(name)
GOTO RECORD 2
? ALLTRIM(name)

* a deleted record is still there until PACK; SET DELETED says whether a program sees it
GO 2
DELETE
? DELETED(), TRANSFORM(RECCOUNT())
SET DELETED ON
? SET("DELETED")
COUNT TO nSeen
? TRANSFORM(nSeen)
SET DELETED OFF
COUNT TO nSeen
? TRANSFORM(nSeen)
GO 2
RECALL
? DELETED()
DELETE ALL FOR pay < 3000
COUNT FOR DELETED() TO nGone
? TRANSFORM(nGone)
RECALL ALL
COUNT FOR DELETED() TO nGone
? TRANSFORM(nGone)

* SET CARRY says whether the record APPEND BLANK makes starts as a copy of the last one
GO BOTTOM
SET CARRY ON
? SET("CARRY")
APPEND BLANK
? "[" + ALLTRIM(name) + "]", TRANSFORM(pay)
SET CARRY OFF
APPEND BLANK
? "[" + ALLTRIM(name) + "]", TRANSFORM(pay)
DELETE ALL FOR EMPTY(name)
PACK
? TRANSFORM(RECCOUNT())

* SET FIELDS narrows a table down to the columns a program is allowed to see
SET FIELDS TO name, pay
SET FIELDS ON
? SET("FIELDS")
? FLDLIST()
SET FIELDS OFF
? FLDLIST()
SET FIELDS TO

* TOTAL adds up the numbers of the records that share a key, in key order
SET ODOMETER TO 50
? TRANSFORM(SET("ODOMETER"))
SORT TO bydept ON dept
SELECT 2
USE bydept
TOTAL ON dept TO totals
USE totals
? TRANSFORM(RECCOUNT())
GO TOP
? ALLTRIM(dept), TRANSFORM(pay)
GO BOTTOM
? ALLTRIM(dept), TRANSFORM(pay)

* DELETE - SQL marks whole rows by a condition rather than by where the pointer stands
SELECT crew
DELETE FROM crew WHERE dept = "SALES"
COUNT FOR DELETED() TO nGone
? TRANSFORM(nGone)

* SET EXCLUSIVE and SET UNIQUE, which a table is opened and indexed under
SET EXCLUSIVE OFF
? SET("EXCLUSIVE"), ISEXCLUSIVE()
SET UNIQUE ON
? SET("UNIQUE")
INDEX ON dept TO onedept
? TRANSFORM(RECCOUNT()), ALLTRIM(dept)
SET UNIQUE OFF
CLOSE ALL
* COVERS: CONTINUE, FLDLIST, DELETE - SQL, GOTO, LOCATE, RECALL, SET CARRY, SET DELETED, SET EXCLUSIVE,
* COVERS: SET FIELDS, SET ODOMETER, SET SAFETY, SET UNIQUE, TOTAL
