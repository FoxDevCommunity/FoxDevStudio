* The older ways of saying things, which the reference keeps for the programs that still say them.
CREATE TABLE orders (code C(4), qty N(4))
INSERT INTO orders VALUES ("A1", 2)
INSERT INTO orders VALUES ("B2", 5)
SELECT 0
CREATE TABLE prices (code C(4), price N(6,2))
INSERT INTO prices VALUES ("A1", 1.50)
INSERT INTO prices VALUES ("B2", 3.00)

* a query that leaves a table of its own behind, open where it ran
SELECT orders.code, qty, price FROM orders, prices ;
    WHERE orders.code = prices.code INTO TABLE billed.dbf
? ALIAS(), RECCOUNT()
SCAN
    ? code, qty * price
ENDSCAN
USE

* JOIN says the same thing the way FoxPro 2.x said it
SELECT orders
JOIN WITH prices TO joined.dbf FOR orders.code = prices.code FIELDS orders.code, qty, price
? ALIAS(), RECCOUNT()
GO TOP
? code, qty, price
USE

* the character screen scrolls a region of itself, and the pointer presses on what it finds
CLEAR
@ 2, 2 SAY "one"
@ 3, 2 SAY "two"
SCROLL 2, 0, 4, 20, 1
? "scrolled"
MOUSE CLICK AT 3, 5
MOUSE DRAG TO 4, 6 LEFT
? MROW(), MCOL()

* a command line handed to the operating system, written the short way
! dir

SELECT orders
USE
SELECT prices
USE
DROP TABLE orders.dbf
DROP TABLE prices.dbf
DROP TABLE billed.dbf
DROP TABLE joined.dbf

* COVERS: !, JOIN, MOUSE, SCROLL, SELECT - SQL
