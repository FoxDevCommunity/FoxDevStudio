* The locking functions asked about a work area that is not the current one, and RLOCK()/LOCK()
* asked to lock a list of records there too.
ON ERROR ?? ""
SET SAFETY OFF

CREATE TABLE staff (name C(10), pay N(8,2))
INSERT INTO staff (name, pay) VALUES ("Nolan", 3200)
INSERT INTO staff (name, pay) VALUES ("Ames", 4100)
INSERT INTO staff (name, pay) VALUES ("Curtis", 2750)

SELECT 2
CREATE TABLE dept (code C(4))
INSERT INTO dept (code) VALUES ("S1")
INSERT INTO dept (code) VALUES ("A1")

SELECT 1

* --- ISREADONLY(), ISEXCLUSIVE(): a table in another work area, by number and by alias
? ISREADONLY(2), ISREADONLY("dept")
? ISEXCLUSIVE("dept"), ISEXCLUSIVE("dept", 1)

* --- FLOCK(), ISFLOCKED(): a table in another work area
? FLOCK(2), ISFLOCKED(2)
? FLOCK("dept"), ISFLOCKED("dept")
UNLOCK IN dept

* --- ISRLOCKED(): nothing locked yet, asked about the current record and about another area
? ISRLOCKED(1)
? ISRLOCKED(1, 2)

* --- RLOCK()/LOCK(): a work area that is not the current one, and a list of records there
? RLOCK(2)
UNLOCK IN dept
? RLOCK("dept")
UNLOCK IN dept
? LOCK(2)
UNLOCK IN dept
? LOCK("dept")
UNLOCK IN dept
? RLOCK("1,2", "dept")
UNLOCK IN dept
? LOCK("1,2", "dept")
UNLOCK IN dept

SELECT 2
USE
SELECT 1
USE

* COVERS: FLOCK, ISEXCLUSIVE, ISFLOCKED, ISREADONLY, ISRLOCKED, LOCK, RLOCK
