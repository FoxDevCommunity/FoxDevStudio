* AUSED, ISMEMOFETCHED, RELATION and TARGET asked about a work area that is not the current
* one, and REQUERY with no argument at all.
ON ERROR ?? ""
SET SAFETY OFF

CREATE TABLE staff (name C(10), code C(4), notes M)
INSERT INTO staff (name, code, notes) VALUES ("Nolan", "S1", "likes bolts")
INSERT INTO staff (name, code, notes) VALUES ("Ames", "A1", "runs admin")

SELECT 2
CREATE TABLE dept (code C(4), title C(10))
INSERT INTO dept (code, title) VALUES ("S1", "Sales")
INSERT INTO dept (code, title) VALUES ("A1", "Admin")
INDEX ON code TAG bycode

SELECT 1
SET RELATION TO code INTO dept ADDITIVE

* --- AUSED(): another data session, which is this one - only one exists here
LOCAL aOpen(1)
? AUSED(aOpen, 1)
? aOpen(1, 1), aOpen(1, 2)

* --- ISMEMOFETCHED(): a memo field in another work area
SELECT 2
? ISMEMOFETCHED(3, 1)
SELECT 1

* --- RELATION(), TARGET(): asked about a work area that is not the current one
SELECT 2
? RELATION(1, 1), TARGET(1, 1)
SELECT 1

* --- REQUERY(): no argument at all, on a table that is not a view
? REQUERY()

SET RELATION TO
SELECT 2
USE
SELECT 1
USE

* COVERS: AUSED, ISMEMOFETCHED, RELATION, REQUERY, TARGET
