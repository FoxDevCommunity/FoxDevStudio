* The commands that ask the user something, run something, or empty a record.
CREATE TABLE parts (code C(4), name C(10), price N(8,2), made D)
INSERT INTO parts VALUES ("A1", "Bolt", 2.50, DATE())
INSERT INTO parts VALUES ("B2", "Nut", 1.75, DATE())

* the mock host answers a dialog with the empty string, which is what a user who
* pressed Cancel leaves behind
cAnswer = "unchanged"
ACCEPT "Your name" TO cAnswer
? "[" + cAnswer + "]"
nValue = 99
INPUT "How many" TO nValue
? VARTYPE(nValue)
cWhere = "x"
GETEXPR "Which records" TO cWhere
? "[" + cWhere + "]"

* a command handed to the operating system, which the mock host does not run
RUN dir
RUN /N notepad.exe

* letting the host catch up
FLUSH
DOEVENTS

* what a program says to itself while it is being worked on
DEBUGOUT "made it to here", 42
SET ASSERTS ON
ASSERT .T. MESSAGE "never seen"
ASSERT .F. MESSAGE "the count is wrong"
SET ASSERTS OFF
ASSERT .F. MESSAGE "not seen with asserts off"

* the record emptied, and one field of it
GO TOP
BLANK
? "[" + code + "]", "[" + ALLTRIM(name) + "]", price
GO BOTTOM
BLANK FIELDS price
? ALLTRIM(name), price

* the old way of looking a key up, and the record editors
INDEX ON code TAG code
GO TOP
FIND B2
? FOUND(), ALLTRIM(name)
EDIT NOWAIT
CHANGE NOWAIT

USE
DROP TABLE parts.dbf

* COVERS: ASSERT, BLANK, CHANGE, DEBUGOUT, DOEVENTS, EDIT, FIND, FLUSH, GETEXPR,
* COVERS: INPUT, RUN, SET ASSERTS
