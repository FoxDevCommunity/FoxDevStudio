* The outer joins: which side's rows survive when the other has no match for them.
SET SAFETY OFF
CREATE TABLE dept (code C(3), title C(10))
INSERT INTO dept (code, title) VALUES ("SAL", "Sales")
INSERT INTO dept (code, title) VALUES ("ADM", "Admin")
INSERT INTO dept (code, title) VALUES ("RND", "Research")
CREATE TABLE staff (name C(8), dcode C(3))
INSERT INTO staff (name, dcode) VALUES ("Nolan", "SAL")
INSERT INTO staff (name, dcode) VALUES ("Ames", "ADM")
INSERT INTO staff (name, dcode) VALUES ("Vance", "XXX")

* an inner join keeps only the pairs
SELECT staff.name, dept.title FROM staff INNER JOIN dept ON staff.dcode = dept.code ;
  ORDER BY 1 INTO CURSOR c1
? TRANSFORM(RECCOUNT())
SCAN
  ? ALLTRIM(name) + "/" + ALLTRIM(title)
ENDSCAN

* a left join keeps every row of the left-hand table
SELECT staff.name, dept.title FROM staff LEFT JOIN dept ON staff.dcode = dept.code ;
  ORDER BY 1 INTO CURSOR c2
? TRANSFORM(RECCOUNT())
SCAN
  ? ALLTRIM(name) + "/[" + ALLTRIM(title) + "]"
ENDSCAN

* a right join keeps every row of the right-hand one, which is the same the other way round
SELECT staff.name, dept.title FROM staff RIGHT JOIN dept ON staff.dcode = dept.code ;
  ORDER BY 2 INTO CURSOR c3
? TRANSFORM(RECCOUNT())
SCAN
  ? "[" + ALLTRIM(name) + "]/" + ALLTRIM(title)
ENDSCAN

* a full join keeps the unmatched rows of both sides
SELECT staff.name, dept.title FROM staff FULL JOIN dept ON staff.dcode = dept.code ;
  ORDER BY 1, 2 INTO CURSOR c4
? TRANSFORM(RECCOUNT())
SCAN
  ? "[" + ALLTRIM(name) + "]/[" + ALLTRIM(title) + "]"
ENDSCAN
USE IN c1
USE IN c2
USE IN c3
USE IN c4
CLOSE ALL
* COVERS: SELECT - SQL
