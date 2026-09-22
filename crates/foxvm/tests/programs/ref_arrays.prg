* Records into an array and back out again, and the table that describes a table's own shape.
SET SAFETY OFF
CREATE TABLE stock (code C(4), name C(8), qty N(5,0))
INSERT INTO stock (code, name, qty) VALUES ("A1", "Bolt", 12)
INSERT INTO stock (code, name, qty) VALUES ("B2", "Nut", 40)

* COPY TO ARRAY fills one row per record and one column per field
COPY TO ARRAY aRows
? TRANSFORM(ALEN(aRows, 1)), TRANSFORM(ALEN(aRows, 2))
? ALLTRIM(aRows(1, 2)), TRANSFORM(aRows(2, 3))
COPY TO ARRAY aOne FIELDS name FOR qty > 20
? TRANSFORM(ALEN(aOne)), ALLTRIM(aOne(1))

* DECLARE is DIMENSION under another name, and the array it makes is one record wide
DECLARE aNew(2, 3)
aNew(1, 1) = "C3"
aNew(1, 2) = "Washer"
aNew(1, 3) = 7
aNew(2, 1) = "D4"
aNew(2, 2) = "Screw"
aNew(2, 3) = 99
APPEND FROM ARRAY aNew
? TRANSFORM(RECCOUNT())
GO 3
? ALLTRIM(code), ALLTRIM(name), TRANSFORM(qty)
GO 4
? ALLTRIM(name), TRANSFORM(qty)

* REPLACE FROM ARRAY writes a row of values across the record the pointer is on
DECLARE aFix(3)
aFix(1) = "Z9"
aFix(2) = "Rivet"
aFix(3) = 5
GO 1
REPLACE FROM ARRAY aFix
? ALLTRIM(code), ALLTRIM(name), TRANSFORM(qty)

* COPY STRUCTURE EXTENDED writes a table whose records are the columns of this one
COPY STRUCTURE EXTENDED TO shape
USE shape IN 0
SELECT shape
? TRANSFORM(RECCOUNT())
GO TOP
? ALLTRIM(field_name), ALLTRIM(field_type), TRANSFORM(field_len)
GO BOTTOM
? ALLTRIM(field_name), ALLTRIM(field_type), TRANSFORM(field_len)
USE
SELECT stock
USE
* COVERS: APPEND FROM ARRAY, COPY STRUCTURE EXTENDED, COPY TO ARRAY, DECLARE,
* COVERS: REPLACE FROM ARRAY
