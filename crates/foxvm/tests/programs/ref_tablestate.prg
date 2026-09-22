* The functions that ask an open table where it is and what it holds.
*
* DBF() answers with the whole path, which is a different directory every run, so what is
* asserted about it is the file name.

CREATE TABLE staff (name C(10), dept C(6), pay N(8,2))
INSERT INTO staff (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO staff (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
INSERT INTO staff (name, dept, pay) VALUES ("Curtis", "SALES", 2750)

* --- is there a table in this work area, in another, and under a name nothing answers to
? USED(), USED("staff"), USED(2), USED("nosuchalias")

* --- what the table is called on disk, and what an empty work area is called
? UPPER(JUSTFNAME(DBF())), UPPER(JUSTFNAME(DBF("staff"))), "[" + DBF(2) + "]"

* --- the columns, as an array: the name, the type, the width and the decimals
? LTRIM(STR(AFIELDS(aCols))), LTRIM(STR(ALEN(aCols, 2)))
? ALLTRIM(aCols(1, 1)), aCols(1, 2), LTRIM(STR(aCols(1, 3))), LTRIM(STR(aCols(1, 4)))
? ALLTRIM(aCols(3, 1)), aCols(3, 2), LTRIM(STR(aCols(3, 3))), LTRIM(STR(aCols(3, 4)))

* --- before the first record, and after the last
GO TOP
? BOF(), EOF(), LTRIM(STR(RECNO()))
SKIP -1
? BOF()
GO BOTTOM
SKIP
? EOF(), LTRIM(STR(RECNO()))
* an empty work area is neither before nor after anything
? BOF(2), EOF(2)
* but a table with no records in it is at both ends at once, and still on record 1
SELECT 2
CREATE TABLE nothing (a C(1))
? BOF(), EOF(), LTRIM(STR(RECNO())), LTRIM(STR(RECCOUNT()))
USE
SELECT staff

* --- the day the table was last written to
? VARTYPE(LUPDATE()), VARTYPE(LUPDATE(2)), EMPTY(LUPDATE(2))

* --- what a field held before this program changed it. Without buffering nothing kept it, and
* the question is refused rather than answered with what is there now.
GO TOP
REPLACE pay WITH 4000
LOCAL oErr, cSaid
TRY
  cSaid = "OLDVAL: " + LTRIM(STR(OLDVAL("pay")))
CATCH TO oErr
  cSaid = "OLDVAL: error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid, LTRIM(STR(pay))

* --- the lock a program takes on the record it is on, and on all of them
? LOCK(), RLOCK()
UNLOCK

USE
? USED(), "[" + DBF() + "]"

* COVERS: AFIELDS, BOF, DBF, LOCK, LUPDATE, OLDVAL, USED
