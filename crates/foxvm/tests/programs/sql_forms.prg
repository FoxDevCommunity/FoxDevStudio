* COVERS: SELECT - SQL, INSERT - SQL, UPDATE - SQL, DELETE - SQL, USE, DO, CREATE TABLE - SQL
*
* Five spellings a real application uses that the reference examples do not: USE IN
* SELECT('alias'), ?lcName parameters in local SQL, INSERT INTO ... SELECT, UNION [ALL], and a
* name in quotes after DO and CREATE TABLE. Every answer below was measured in Visual FoxPro 9.
CREATE CURSOR parts (code C(3), qty N(3))
INSERT INTO parts VALUES ("AAA", 1)
INSERT INTO parts VALUES ("BBB", 2)
INSERT INTO parts VALUES ("CCC", 3)

* USE IN SELECT(): closes the table when it is open, and does nothing when it is not
CREATE CURSOR other (a C(1))
? "used before", USED("other")
USE IN SELECT("other")
? "used after", USED("other")
USE IN SELECT("other")
? "still here", ALIAS()

* ?name is an expression, read as it would be without the mark; ?m.name says a variable.
* (Measured too, and not written here because this runtime lets a LOCAL shadow a field: a
* name that is both a field of a source and a variable is the field in the product.)
LOCAL lcCode
lcCode = "BBB"
SELECT qty FROM parts WHERE parts.code = ?lcCode INTO CURSOR q1
? "param variable", TRANSFORM(RECCOUNT()), TRANSFORM(qty), ALIAS()
SELECT qty FROM parts WHERE parts.code = ?m.lcCode INTO CURSOR q1
? "param m.", TRANSFORM(RECCOUNT()), TRANSFORM(qty)

* INSERT ... SELECT: to the fields named, in the order named
CREATE CURSOR bigger (qty N(3), code C(3))
SELECT parts
INSERT INTO bigger (code, qty) SELECT code, qty FROM parts WHERE qty > 1
? "insert select", TRANSFORM(RECCOUNT("bigger")), ALIAS()
SELECT bigger
SCAN
  ? "  row", code, TRANSFORM(qty)
ENDSCAN

* and without a field list, to the table's fields in order
CREATE CURSOR copy (code C(3), qty N(3))
SELECT parts
INSERT INTO copy SELECT code, qty FROM parts WHERE qty = 1
? "insert select whole", TRANSFORM(RECCOUNT("copy")), copy.code, TRANSFORM(copy.qty), ALIAS()

* UNION ALL keeps every row; the columns are the first SELECT's
SELECT 1 AS norder, "top " AS label, code FROM parts WHERE qty = 3 ;
  UNION ALL ;
  SELECT 2 AS norder, "rest" AS label, code FROM parts WHERE qty < 3 ;
  ORDER BY norder, code INTO CURSOR q2
? "union all", TRANSFORM(RECCOUNT("q2")), ALIAS()
SCAN
  ? "  row", TRANSFORM(norder), label, code
ENDSCAN

* UNION without ALL folds the rows the two sides have in common
SELECT code FROM parts WHERE NOT DELETED() UNION SELECT code FROM parts WHERE qty > 1 INTO CURSOR q3
? "union", TRANSFORM(RECCOUNT("q3"))
SELECT code FROM parts WHERE NOT DELETED() UNION ALL SELECT code FROM parts WHERE qty > 1 INTO CURSOR q4
? "union all again", TRANSFORM(RECCOUNT("q4"))

* INTO ARRAY leaves the program where it stood
SELECT parts
SELECT code FROM parts WHERE qty = 1 UNION ALL SELECT code FROM parts WHERE qty = 3 INTO ARRAY laCodes
? "into array", TRANSFORM(ALEN(laCodes)), laCodes(1), laCodes(2), ALIAS()

* a quoted name after DO
DO "helper" WITH "x"

PROCEDURE helper
  LPARAMETERS c
  ? "helper ran", c
ENDPROC
