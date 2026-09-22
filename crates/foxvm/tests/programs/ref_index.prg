* Wave 3 of the language reference: the compound index beside a table.
CREATE TABLE staff (name C(10), dept C(6), pay N(8,2))
INSERT INTO staff (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO staff (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
INSERT INTO staff (name, dept, pay) VALUES ("Curtis", "SALES", 2750)
INSERT INTO staff (name, dept, pay) VALUES ("Bell", "ADMIN", 5000)
INDEX ON name TAG byname
? ORDER(), TAGCOUNT(), TAG(1), KEY(1), TAGNO(), TAGNO("byname")
? DESCENDING(1), UNIQUE(1), CANDIDATE(1), PRIMARY(1)
GO TOP
? ALLTRIM(name)
SKIP
? ALLTRIM(name)
GO BOTTOM
? ALLTRIM(name), RECNO()
SKIP
? EOF()
* SEEK down the controlling order, and what a miss does
SEEK "Curtis"
? FOUND(), ALLTRIM(name)
SEEK "Zeno"
? FOUND(), EOF()
SET NEAR ON
SEEK "B"
? FOUND(), ALLTRIM(name)
SET NEAR OFF
? SEEK("Ames"), ALLTRIM(name)
? INDEXSEEK("Bell", .F.), ALLTRIM(name)
? INDEXSEEK("Bell", .T.), ALLTRIM(name)
* a second tag, and the one just built is the order
INDEX ON dept TAG bydept
? TAGCOUNT(), ORDER()
GO TOP
? ALLTRIM(dept), ALLTRIM(name)
INDEX ON dept TAG bydeptu UNIQUE
? UNIQUE(TAGNO("bydeptu")), TAGCOUNT()
nRows = 0
SCAN
   nRows = nRows + 1
ENDSCAN
? nRows
LOCAL aTags(1)
? ATAGINFO(aTags)
? aTags(1, 1), aTags(1, 2), aTags(1, 3), aTags(1, 5)
* a record put somewhere else in the order by what was written to it
SET ORDER TO byname
GO TOP
REPLACE name WITH "Zane"
GO BOTTOM
? ALLTRIM(name)
GO TOP
? ALLTRIM(name)
* a filter over the order, and a work area that follows this one
SET FILTER TO pay > 3000
? FILTER()
GO TOP
? ALLTRIM(name)
SKIP
? ALLTRIM(name)
SET FILTER TO
SELECT 2
CREATE TABLE tasks (owner C(10), task C(12))
INSERT INTO tasks (owner, task) VALUES ("Bell", "Audit")
INSERT INTO tasks (owner, task) VALUES ("Zane", "Payroll")
INDEX ON owner TAG byowner
SELECT 1
SET RELATION TO name INTO tasks
? RELATION(1), TARGET(1)
GO TOP
? ALLTRIM(name), ALLTRIM(tasks.task), FOUND("tasks")
GO BOTTOM
? ALLTRIM(name), ALLTRIM(tasks.task)
SET RELATION OFF INTO tasks
? RELATION(1) == ""
SELECT 2
USE
SELECT 1
* which way the order runs, and no order at all
SET ORDER TO byname DESCENDING
GO TOP
? ALLTRIM(name)
SET ORDER TO
? ORDER() == "", TAGNO()
GO TOP
? ALLTRIM(name)
SET ORDER TO 1
? ORDER()
REINDEX
? TAGCOUNT()
DELETE TAG bydeptu
? TAGCOUNT()
DELETE TAG ALL
? TAGCOUNT(), ORDER() == "", CDX(1) == "", NDX(1) == ""
USE
* COVERS: ALLTRIM, ATAGINFO, CANDIDATE, CDX, CREATE TABLE, DELETE TAG, DESCENDING, EOF, FILTER,
* COVERS: FOUND, GO, INDEX, INDEXSEEK, INSERT, KEY, LOCAL, NDX, ORDER, PRIMARY, RECNO, REINDEX,
* COVERS: RELATION, REPLACE, SCAN, SEEK, SELECT, SET FILTER, SET NEAR, SET ORDER, SET RELATION,
* COVERS: SKIP, TAG, TAGCOUNT, TAGNO, TARGET, UNIQUE, USE
