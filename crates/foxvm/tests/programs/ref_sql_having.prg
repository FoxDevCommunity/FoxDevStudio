* COVERS: SELECT - SQL
*
* HAVING filters the groups a query has folded, which is why it may name an aggregate where a
* WHERE clause may not. Every answer below was measured in Visual FoxPro 9.
CREATE CURSOR sales (city C(3), country C(3), amt N(5))
INSERT INTO sales VALUES ("A", "USA", 1)
INSERT INTO sales VALUES ("A", "USA", 2)
INSERT INTO sales VALUES ("B", "USA", 3)
INSERT INTO sales VALUES ("C", "MEX", 4)
INSERT INTO sales VALUES ("C", "MEX", 5)
INSERT INTO sales VALUES ("C", "MEX", 6)
INSERT INTO sales VALUES ("D", "CAN", 7)

* the groups with more than one record in them
DO try WITH "counted", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) > 1 INTO CURSOR q1"

* an aggregate the select list never asked for, and every aggregate there is
DO try WITH "max over five", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING MAX(amt) > 5 INTO CURSOR q1"
DO try WITH "min over three", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING MIN(amt) > 3 INTO CURSOR q1"
DO try WITH "average over four", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING AVG(amt) > 4 INTO CURSOR q1"
DO try WITH "counted by column", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING COUNT(amt) > 1 INTO CURSOR q1"
DO try WITH "summed over five", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING SUM(amt) > 5 INTO CURSOR q1"

* the name AS gave a column is visible in it, whether the column is an aggregate or not
DO try WITH "sum over five", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING s > 5 INTO CURSOR q1"
DO try WITH "named C", "SELECT city AS where_, COUNT(*) AS n FROM sales GROUP BY city HAVING where_ = 'C' INTO CURSOR q1"
DO try WITH "renamed plain column", "SELECT city AS town, COUNT(*) AS n FROM sales GROUP BY city HAVING town = 'C' INTO CURSOR q1"

* WHERE throws records away first, so HAVING sees only the groups what is left makes
DO try WITH "usa, counted", "SELECT city, COUNT(*) AS n FROM sales WHERE country = 'USA' GROUP BY city HAVING COUNT(*) > 1 INTO CURSOR q1"

* a grouped column, written plainly and written with the table in front of it
DO try WITH "after A", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING city > 'A' INTO CURSOR q1"
DO try WITH "after A, qualified", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING sales.city > 'A' INTO CURSOR q1"

* a grouped column inside something else, and an aggregate inside something else
DO try WITH "upper of a grouped column", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING UPPER(city) = 'C' INTO CURSOR q1"
DO try WITH "a grouped column in arithmetic", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING ALLTRIM(city) + '!' = 'C!' INTO CURSOR q1"
DO try WITH "two or three", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING BETWEEN(COUNT(*), 2, 3) INTO CURSOR q1"
DO try WITH "doubled", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING n * 2 > 4 INTO CURSOR q1"
DO try WITH "an aggregate of a grouped column", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING MAX(city) = 'C' INTO CURSOR q1"

* two aggregates at once, and a group there is nothing left of
DO try WITH "counted and summed", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) > 1 AND SUM(amt) > 10 INTO CURSOR q1"
DO try WITH "none of them", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) > 99 INTO CURSOR q1"

* nothing of a record's own: a constant, a function of none of it, a variable of the program's
DO try WITH "a constant", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING 1 = 1 INTO CURSOR q1"
DO try WITH "a function of no columns", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING LEN('ab') = 2 INTO CURSOR q1"
DO try WITH "at least the variable", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) >= nFloor INTO CURSOR q1"
DO try WITH "through a function", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING mine(COUNT(*)) INTO CURSOR q1"

* a column the query groups by its number, reached by the name AS gave it and written out again
DO try WITH "grouped by number, by its AS name", "SELECT LEFT(city,1) AS c, COUNT(*) AS n FROM sales GROUP BY 1 HAVING c = 'C' INTO CURSOR q1"
DO try WITH "grouped by number, written out again", "SELECT LEFT(city,1) AS c, COUNT(*) AS n FROM sales GROUP BY 1 HAVING LEFT(city,1) = 'C' INTO CURSOR q1"

* with no GROUP BY and nothing aggregated, HAVING is WHERE by another name: it filters records,
* and it may name a column the select list left out
DO try WITH "no groups", "SELECT city FROM sales HAVING amt > 5 INTO CURSOR q1"
DO try WITH "where and having", "SELECT city FROM sales WHERE country = 'MEX' HAVING amt > 5 INTO CURSOR q1"

* the query folds the whole table into one row and groups nothing: HAVING tests that one row
DO try WITH "whole table, kept", "SELECT COUNT(*) AS n, SUM(amt) AS s FROM sales HAVING COUNT(*) > 3 INTO CURSOR q1"
DO try WITH "whole table, dropped", "SELECT COUNT(*) AS n, SUM(amt) AS s FROM sales HAVING COUNT(*) > 99 INTO CURSOR q1"

* it comes before ORDER BY, and TOP counts what is left of the groups
DO try WITH "kept, then ordered", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING COUNT(*) < 3 ORDER BY 2 DESC INTO CURSOR q1"
DO try WITH "kept, ordered, topped", "SELECT TOP 2 city, SUM(amt) AS s FROM sales GROUP BY city HAVING COUNT(*) < 3 ORDER BY 2 DESC INTO CURSOR q1"
DO try WITH "distinct and kept", "SELECT DISTINCT country, COUNT(*) AS n FROM sales GROUP BY country HAVING COUNT(*) > 1 INTO CURSOR q1"

* what it may not name: a column that is neither grouped nor inside an aggregate, however it is
* written, and a name AS gave that a field of one of the sources is called too
DO try WITH "ungrouped", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING country = 'MEX' INTO CURSOR q1"
DO try WITH "ungrouped, in a function", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING LEFT(country,1) = 'M' INTO CURSOR q1"
DO try WITH "ungrouped, in arithmetic", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING country + '!' = 'MEX!' INTO CURSOR q1"
DO try WITH "shadowed alias", "SELECT city AS amt, COUNT(*) AS n FROM sales GROUP BY city HAVING amt > 'A' INTO CURSOR q1"
DO try WITH "ungrouped, whole table folded", "SELECT SUM(amt) AS s FROM sales HAVING city = 'C' INTO CURSOR q1"

* an aggregate in a HAVING clause over a query that folds nothing at all
DO try WITH "no group by", "SELECT city FROM sales HAVING COUNT(*) > 3 INTO CURSOR q1"

* a query of its own is not one of the things it may name
DO try WITH "a subquery", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING city IN (SELECT country FROM sales) INTO CURSOR q1"

* and what it comes to has to be an answer to which groups to keep
DO try WITH "not a logical", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) INTO CURSOR q1"

* SET ENGINEBEHAVIOR 70 is the older rules for GROUP BY; this is what it does to HAVING. A
* column that is neither grouped nor aggregated is no longer refused, however it is written,
* and the whole table folding into one group is no longer a missing GROUP BY. What a name the
* select list gave with AS means does not change: a field of one of the sources still wins,
* and under these rules winning means the field is read rather than the query refused.
CREATE CURSOR spread (city C(3), country C(3), amt N(5))
INSERT INTO spread VALUES ("A", "USA", 1)
INSERT INTO spread VALUES ("A", "CAN", 2)
INSERT INTO spread VALUES ("B", "USA", 3)
INSERT INTO spread VALUES ("C", "MEX", 4)
INSERT INTO spread VALUES ("C", "MEX", 5)
INSERT INTO spread VALUES ("C", "MEX", 6)
INSERT INTO spread VALUES ("D", "CAN", 7)
SET ENGINEBEHAVIOR 70
DO try WITH "70: ungrouped", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING country = 'MEX' INTO CURSOR q1"
DO try WITH "70: counted", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) > 1 INTO CURSOR q1"
DO try WITH "70: a grouped column", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING city > 'B' INTO CURSOR q1"
DO try WITH "70: an AS name", "SELECT city, SUM(amt) AS s FROM sales GROUP BY city HAVING s > 5 INTO CURSOR q1"
DO try WITH "70: a variable", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) >= nFloor INTO CURSOR q1"
DO try WITH "70: which record the group answers with", "SELECT city, COUNT(*) AS n FROM spread GROUP BY city HAVING country = 'USA' INTO CURSOR q1"
DO try WITH "70: ungrouped, in a function", "SELECT city, COUNT(*) AS n FROM spread GROUP BY city HAVING LEFT(country,1) = 'U' INTO CURSOR q1"
DO try WITH "70: ungrouped, in arithmetic", "SELECT city, COUNT(*) AS n FROM spread GROUP BY city HAVING country + '!' = 'USA!' INTO CURSOR q1"
DO try WITH "70: ungrouped, qualified", "SELECT city, COUNT(*) AS n FROM spread GROUP BY city HAVING spread.country = 'USA' INTO CURSOR q1"
DO try WITH "70: ungrouped and an aggregate", "SELECT city, COUNT(*) AS n FROM spread GROUP BY city HAVING country = 'USA' AND COUNT(*) > 1 INTO CURSOR q1"
DO try WITH "70: an ungrouped number", "SELECT city, COUNT(*) AS n FROM spread GROUP BY city HAVING amt > 4 INTO CURSOR q1"
DO try WITH "70: shadowed alias", "SELECT city AS amt, COUNT(*) AS n FROM sales GROUP BY city HAVING amt > 'A' INTO CURSOR q1"
DO try WITH "70: ungrouped, whole table folded", "SELECT SUM(amt) AS s FROM spread HAVING city = 'C' INTO CURSOR q1"
DO try WITH "70: no group by", "SELECT city FROM spread HAVING COUNT(*) > 3 INTO CURSOR q1"
DO try WITH "70: a subquery", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING city IN (SELECT country FROM sales) INTO CURSOR q1"
DO try WITH "70: not a logical", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING COUNT(*) INTO CURSOR q1"
SET ENGINEBEHAVIOR 90
DO try WITH "90 again: ungrouped", "SELECT city, COUNT(*) AS n FROM sales GROUP BY city HAVING country = 'MEX' INTO CURSOR q1"

USE IN sales
USE IN spread
RETURN

PROCEDURE mine(n)
RETURN n > 2

PROCEDURE try(cLabel, cSql)
LOCAL oErr, nFloor
nFloor = 2
TRY
   &cSql
   DO show WITH cLabel
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
