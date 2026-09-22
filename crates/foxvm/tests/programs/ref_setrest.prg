* The rest of the settings: the ones about colour, the keyboard, help, and the ones that only
* mean anything while a table is open. SET ALTERNATE is not here - it is what the harness
* captures a program's output with, and a program that changes it stops being measurable.
SET SAFETY OFF
SET TALK OFF
? SET("TALK")
SET ESCAPE OFF
? SET("ESCAPE")
SET TOPIC TO "Fox"
? SET("TOPIC")
SET TOPIC ID TO 5
SET WINDOW OF MEMO TO
SET EVENTLIST TO Click, DblClick
SET FUNCTION 2 TO "hi"
SET BORDER TO SINGLE
SET COLOR TO W+/B
SET COLOR OF SCHEME 1 TO W+/B
SET COLOR OF MESSAGES TO W/N
? "settings taken"

* SET KEY narrows a table to a range of its index, and SET NOCPTRANS says a field is bytes
CREATE TABLE crew (name C(10), dept C(6))
INSERT INTO crew (name, dept) VALUES ("Ames", "ADMIN")
INSERT INTO crew (name, dept) VALUES ("Bell", "ADMIN")
INSERT INTO crew (name, dept) VALUES ("Curtis", "SALES")
INSERT INTO crew (name, dept) VALUES ("Nolan", "SALES")
INDEX ON name TAG name
SET KEY TO "B", "D"
COUNT TO nInRange
? TRANSFORM(nInRange)
* the reference calls the second expression the top of a range. It is not one: what a program
* is left with is the records matching the first expression, whatever the second says
SET KEY TO "Bell", "Nolan"
COUNT TO nInRange
? TRANSFORM(nInRange)
SET KEY TO "S"
COUNT TO nInRange
? TRANSFORM(nInRange)
SET KEY TO
COUNT TO nAll
? TRANSFORM(nAll)
SET NOCPTRANS TO dept
? TRANSFORM(RECCOUNT())

* a relation between two tables, and the two commands that undo half of it
SELECT 0
CREATE TABLE pay (dept C(6), rate N(6,2))
INSERT INTO pay (dept, rate) VALUES ("ADMIN", 12)
INSERT INTO pay (dept, rate) VALUES ("SALES", 9)
INDEX ON dept TAG dept
SELECT crew
SET ORDER TO name
SET RELATION TO dept INTO pay
GO TOP
? ALLTRIM(name), TRANSFORM(pay.rate)
SET SKIP TO pay
? ALLTRIM(name), TRANSFORM(pay.rate)
SET SKIP TO
SET RELATION OFF INTO pay
GO TOP
? ALLTRIM(RELATION(1))
CLOSE ALL
* COVERS: SET BORDER, SET COLOR OF, SET COLOR OF SCHEME, SET COLOR TO, SET ESCAPE,
* COVERS: SET EVENTLIST, SET FUNCTION, SET KEY, SET NOCPTRANS, SET RELATION OFF, SET SKIP,
* COVERS: SET TALK, SET TOPIC, SET TOPIC ID, SET WINDOW OF MEMO
