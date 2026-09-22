* Wave 4 of the language reference: the commands that work on a table as a whole.
CREATE TABLE staff (name C(10), dept C(6), pay N(8,2))
INSERT INTO staff (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO staff (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
INSERT INTO staff (name, dept, pay) VALUES ("Curtis", "SALES", 2750)
INSERT INTO staff (name, dept, pay) VALUES ("Bell", "ADMIN", 5000)
* a record away from the table and back again
GO TOP
SCATTER TO aRow
? ALEN(aRow), ALLTRIM(aRow(1)), ALLTRIM(aRow(2)), aRow(3)
aRow(3) = 3400
GATHER FROM aRow
? ALLTRIM(name), pay
SCATTER MEMVAR
? ALLTRIM(m.name), m.pay
SCATTER NAME oRow
? ALLTRIM(oRow.name), oRow.dept
SCATTER FIELDS name, pay TO aTwo
? ALEN(aTwo)
SCATTER BLANK TO aBlank
? "[" + aBlank(2) + "]", aBlank(3)
* what the records add up to
COUNT TO nAll
? nAll
COUNT FOR pay > 3000 TO nRich
? nRich
SUM pay TO nTotal
? nTotal
AVERAGE pay TO nMean
? nMean
CALCULATE CNT(), SUM(pay), AVG(pay), MIN(pay), MAX(pay) TO c, s, a, lo, hi
? c, s, ROUND(a, 2), lo, hi
CALCULATE MIN(name), MAX(name) TO cFirst, cLast
? ALLTRIM(cFirst), ALLTRIM(cLast)
CALCULATE SUM(pay) FOR dept = "ADMIN" TO nAdmin
? nAdmin
* a table made from a table
COPY TO backup.dbf
COPY STRUCTURE TO shape.dbf
SORT TO bypay.dbf ON pay /D
COPY TO rows.txt TYPE SDF
SELECT 2
USE bypay.dbf
? RECCOUNT(), ALLTRIM(name), pay
GO BOTTOM
? ALLTRIM(name)
USE shape.dbf
? RECCOUNT(), FCOUNT()
USE backup.dbf
? RECCOUNT()
* records from another file
ZAP
? RECCOUNT()
APPEND FROM rows.txt TYPE SDF
? RECCOUNT()
GO TOP
? ALLTRIM(name), pay
APPEND FROM bypay.dbf FOR pay > 4000
? RECCOUNT()
* the records that are marked deleted, and what is left when they go
GO TOP
DELETE
? DELETED(), RECCOUNT()
PACK
? RECCOUNT(), DELETED()
* the shape of a table, changed
ALTER TABLE backup.dbf ADD COLUMN city C(12)
? FCOUNT(), FIELD(4)
GO TOP
REPLACE city WITH "Bristol"
? ALLTRIM(city)
ALTER TABLE backup.dbf RENAME COLUMN city TO town
? FIELD(4)
ALTER TABLE backup.dbf DROP COLUMN town
? FCOUNT()
* the locks one program holds
? RLOCK(), ISRLOCKED()
UNLOCK
? ISRLOCKED()
? FLOCK(), ISFLOCKED()
UNLOCK ALL
? ISFLOCKED()
USE
DROP TABLE backup.dbf
DROP TABLE bypay.dbf
DROP TABLE shape.dbf
? FILE("backup.dbf")
* COVERS: ALEN, ALTER TABLE, APPEND FROM, AVERAGE, CALCULATE, COPY STRUCTURE, COPY TO, COUNT,
* COVERS: CREATE TABLE, DELETE, DELETED, DROP TABLE, FCOUNT, FIELD, FILE, FLOCK, GATHER, GO,
* COVERS: INSERT, ISFLOCKED, ISRLOCKED, PACK, RECCOUNT, RLOCK, ROUND, SCATTER, SELECT, SORT,
* COVERS: SUM, UNLOCK, USE, ZAP
