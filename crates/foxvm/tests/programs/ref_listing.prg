* What a program can ask the runtime to write out, and the SQL spelling of REPLACE.
CREATE TABLE parts (code C(4), name C(10), price N(8,2))
INSERT INTO parts (code, name, price) VALUES ("A1", "Bolt", 2.50)
INSERT INTO parts (code, name, price) VALUES ("B2", "Nut", 1.75)
INSERT INTO parts (code, name, price) VALUES ("C3", "Washer", 0.40)
* the records themselves, all of them and some of them
LIST
LIST code, price FOR price > 1
DISPLAY FIELDS name
* and what the runtime holds
LIST STRUCTURE
LIST STATUS
LIST PROCEDURES
LIST CONNECTIONS
* the SQL spelling of REPLACE
UPDATE parts SET price = 9.99 WHERE code = "B2"
GO 2
? price
UPDATE parts SET name = "Same"
GO TOP
? ALLTRIM(name)
USE
DROP TABLE parts.dbf
* COVERS: CREATE TABLE, DISPLAY, DISPLAY STRUCTURE, DROP TABLE, GO, INSERT, LIST,
* COVERS: DISPLAY STATUS, LIST CONNECTIONS, LIST PROCEDURES, UPDATE - SQL, USE
