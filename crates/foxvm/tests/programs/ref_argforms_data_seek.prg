* The searching functions asked to leave the record pointer where it is, to search a work area
* that is not the current one, and to search without an index at all - the forms nobody had
* measured together.
ON ERROR ?? ""
SET SAFETY OFF

CREATE TABLE staff (name C(10), dept C(6), pay N(8,2))
INSERT INTO staff (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO staff (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
INSERT INTO staff (name, dept, pay) VALUES ("Curtis", "SALES", 2750)
INDEX ON name TAG byname
INDEX ON dept TAG bydept
INDEX ON name TAG bysales FOR dept = "SALES"

SELECT 2
CREATE TABLE dept (code C(4), title C(10))
INSERT INTO dept (code, title) VALUES ("A1", "Admin")
INSERT INTO dept (code, title) VALUES ("S1", "Sales")
INDEX ON code TAG bycode
INDEX ON title TAG bytitle

SELECT 1

* --- FOR(): the controlling order's own condition, and a tag in another work area
? FOR()
? FOR(1, 2)
? FOR("", 1, 2)

* --- SEEK(): a work area that is not the current one, and a named tag there too
? SEEK("A1", 2), ALLTRIM(dept.title)
GO TOP IN dept
? SEEK("Sales", 2, "bytitle"), ALLTRIM(dept.title)

* --- INDEXSEEK(): the key alone, leaving the pointer put; then a work area, and a tag there
GO TOP
? INDEXSEEK("Nolan"), RECNO(), ALLTRIM(name)
? INDEXSEEK("A1", .T., 2), ALLTRIM(dept.title)
GO TOP IN dept
? INDEXSEEK("Sales", .T., 2, "bytitle"), ALLTRIM(dept.title)

* --- KEYMATCH(): the key and an index number, then a work area too
? KEYMATCH("Ames", 1)
? KEYMATCH("ZZ", 1)
? KEYMATCH("A1", 1, 2)

* --- LOOKUP(): the return field, key and searched field, with no tag to go down at all
SELECT 2
? ALLTRIM(LOOKUP(title, "A1", code))
? LOOKUP(title, "ZZ", code)

SELECT 2
USE
SELECT 1
USE

* COVERS: FOR, INDEXSEEK, KEYMATCH, LOOKUP, SEEK
