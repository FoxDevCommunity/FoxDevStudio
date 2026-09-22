* COVERS: SELECT - SQL
*
* TOP n PERCENT takes a share of the rows the query came to rather than a count of them. Every
* answer below was measured in Visual FoxPro 9.
CREATE CURSOR nums (n N(5), tag C(1))
INSERT INTO nums VALUES (1, "a")
INSERT INTO nums VALUES (2, "b")
INSERT INTO nums VALUES (3, "c")
INSERT INTO nums VALUES (4, "d")
INSERT INTO nums VALUES (5, "e")
INSERT INTO nums VALUES (6, "f")
INSERT INTO nums VALUES (7, "g")

* the share is rounded up: half of seven rows is four, and seven tenths of a row is one
SELECT TOP 50 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "half"
SELECT TOP 10 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "a tenth"
SELECT TOP 30 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "under a third"
SELECT TOP 14 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "just under a seventh"
SELECT TOP 15 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "just over a seventh"
SELECT TOP 99 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "nearly all"
SELECT TOP 50.5 PERCENT * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "a fraction of a percent over half"

* which end of the order it takes is the order's own business
SELECT TOP 30 PERCENT * FROM nums ORDER BY n DESC INTO CURSOR q1
DO show WITH "the top three"

* the share is of the rows the query has after grouping, not of the records it read
SELECT TOP 50 PERCENT tag, SUM(n) AS s FROM nums GROUP BY tag ORDER BY 2 DESC INTO CURSOR q1
DO show WITH "half the groups"

* rows tied at the cut are not kept: a count is a count
CREATE CURSOR ties (n N(5), tag C(1))
INSERT INTO ties VALUES (1, "a")
INSERT INTO ties VALUES (1, "b")
INSERT INTO ties VALUES (2, "c")
INSERT INTO ties VALUES (2, "d")
INSERT INTO ties VALUES (3, "e")
SELECT TOP 3 * FROM ties ORDER BY n INTO CURSOR q1
DO show WITH "three of five, tied at the cut"
SELECT TOP 1 * FROM ties ORDER BY n INTO CURSOR q1
DO show WITH "one of five, tied at the cut"
SELECT TOP 50 PERCENT * FROM ties ORDER BY n INTO CURSOR q1
DO show WITH "half of five, tied at the cut"
SELECT TOP 20 PERCENT * FROM ties ORDER BY n INTO CURSOR q1
DO show WITH "a fifth of five, tied at the cut"

* a plain TOP may ask for more rows than there are; a percentage may not be the whole hundred
SELECT TOP 20 * FROM nums ORDER BY n INTO CURSOR q1
DO show WITH "twenty of seven"

* a hundred rows, so that a percentage lands on a whole number of them and a fraction of a
* percent can be seen to round the same way as any other fraction
CREATE CURSOR many (n N(5))
LOCAL i
FOR i = 1 TO 100
   INSERT INTO many VALUES (i)
ENDFOR
DO counts WITH "one percent of a hundred", 1
DO counts WITH "a tenth of a percent of a hundred", 0.1
DO counts WITH "a third of a hundred", 33.3
DO counts WITH "just under all of a hundred", 99.9
DO counts WITH "twelve and a bit of a hundred", 12.3
DO counts WITH "the smallest share there is", 0.01

DO refuses WITH "none of it", 0, .T.
DO refuses WITH "all of it", 100, .T.
DO refuses WITH "more than all of it", 150, .T.
DO refuses WITH "no rows at all", 0, .F.

* a WHERE that lets nothing through leaves nothing for TOP to take a share of
SELECT TOP 50 PERCENT * FROM nums WHERE n > 99 ORDER BY n INTO CURSOR q1
DO show WITH "half of nothing"
SELECT TOP 3 * FROM nums WHERE n > 99 ORDER BY n INTO CURSOR q1
DO show WITH "three of nothing"

* DISTINCT is written before TOP, and the share is of the rows that are left once it has had
* them: six records, three of them distinct, and half of that is two
CREATE CURSOR dupes (tag C(1))
INSERT INTO dupes VALUES ("a")
INSERT INTO dupes VALUES ("a")
INSERT INTO dupes VALUES ("b")
INSERT INTO dupes VALUES ("b")
INSERT INTO dupes VALUES ("c")
INSERT INTO dupes VALUES ("c")
SELECT DISTINCT TOP 50 PERCENT tag FROM dupes ORDER BY 1 INTO CURSOR q1
DO show WITH "half the distinct tags"
SELECT DISTINCT TOP 2 tag FROM dupes ORDER BY 1 INTO CURSOR q1
DO show WITH "two distinct tags"
SELECT TOP 50 PERCENT tag FROM dupes ORDER BY 1 INTO CURSOR q1
DO show WITH "half the tags"
TRY
   SELECT TOP 2 DISTINCT tag FROM dupes ORDER BY 1 INTO CURSOR q1
   DO show WITH "top before distinct"
CATCH TO oErr
   ? "top before distinct:", oErr.ErrorNo, oErr.Message
ENDTRY

* TOP says which rows of an order, so there has to be one, percentage or not
TRY
   SELECT TOP 50 PERCENT * FROM nums INTO CURSOR q1
CATCH TO oErr
   ? "unordered:", oErr.ErrorNo, oErr.Message
ENDTRY
TRY
   SELECT TOP 3 * FROM nums INTO CURSOR q1
CATCH TO oErr
   ? "unordered count:", oErr.ErrorNo, oErr.Message
ENDTRY

USE IN nums
USE IN dupes
USE IN ties
USE IN many
RETURN

PROCEDURE counts(cLabel, nPercent)
SELECT TOP (nPercent) PERCENT * FROM many ORDER BY n INTO CURSOR q1
? cLabel + ", rows:", RECCOUNT()
USE IN q1

PROCEDURE refuses(cLabel, nHow, lPercent)
LOCAL oErr
TRY
   IF lPercent
      SELECT TOP (nHow) PERCENT * FROM nums ORDER BY n INTO CURSOR q1
   ELSE
      SELECT TOP (nHow) * FROM nums ORDER BY n INTO CURSOR q1
   ENDIF
   ? cLabel + ": no complaint"
CATCH TO oErr
   ? cLabel + ":", oErr.ErrorNo, oErr.Message
ENDTRY

PROCEDURE show(cLabel)
LOCAL i, cRow
SELECT q1
? cLabel + ", rows:", RECCOUNT()
SCAN
   cRow = ""
   FOR i = 1 TO FCOUNT()
      cRow = cRow + "[" + ALLTRIM(TRANSFORM(EVALUATE(FIELD(i)))) + "]"
   ENDFOR
   ? "  " + cRow
ENDSCAN
USE IN q1
