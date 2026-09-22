* The work-area functions asked about a table other than the current one, by number and by
* alias - the form nobody had measured, and where SET("HELP", 1) shaped bug lived: an optional
* second argument nobody wired past the first.
ON ERROR ?? ""
SET SAFETY OFF

CREATE TABLE staff (name C(10), dept C(6), pay N(8,2))
INSERT INTO staff (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO staff (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
SET FILTER TO pay > 3000
GO TOP

SELECT 2
CREATE TABLE dept (code C(4), title C(10))
INSERT INTO dept (code, title) VALUES ("S1", "Sales")
DELETE

SELECT 1

* --- SELECT(): the current work area, with no argument at all
? SELECT()
? SELECT("dept"), SELECT("nosuchalias")

* --- ALIAS(nWorkArea | cTableAlias): the alias by number and by name
? ALIAS(2), ALIAS("dept"), ALIAS(3)

* --- FCOUNT(nWorkArea | cTableAlias)
? FCOUNT(2), FCOUNT("dept")

* --- FIELD(nFieldNumber, nWorkArea | cTableAlias)
? FIELD(1, 2), FIELD(2, "dept")

* --- FLDLIST(nWorkArea | cTableAlias): what SET FIELDS named there, which is nothing
? FLDLIST(2), FLDLIST("dept")

* --- HEADER(nWorkArea | cTableAlias)
? HEADER(2) > 0, HEADER("dept") > 0

* --- RECSIZE(nWorkArea | cTableAlias)
? RECSIZE(2), RECSIZE("dept")

* --- FILTER(nWorkArea | cTableAlias): the filter in a work area that is not the current one
? FILTER(1), FILTER("staff")

* --- DELETED(nWorkArea | cTableAlias): a record marked for deletion in another work area
? DELETED(2), DELETED("dept")

* --- CPDBF(nWorkArea | cTableAlias)
? CPDBF(2) == CPDBF("dept")

* --- FSIZE(cFieldName, nWorkArea | cTableAlias)
? FSIZE("code", 2), FSIZE("title", "dept")

* --- AFIELDS(ArrayName, nWorkArea | cTableAlias)
LOCAL aCols(1)
? AFIELDS(aCols, 2), AFIELDS(aCols, "dept")
? ALLTRIM(aCols(1, 1)), ALLTRIM(aCols(2, 1))

SELECT 2
USE
SELECT 1
SET FILTER TO
USE

* COVERS: AFIELDS, ALIAS, CPDBF, DELETED, FCOUNT, FIELD, FLDLIST, FSIZE, FILTER, HEADER, RECSIZE, SELECT
