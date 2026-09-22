* Almost anywhere Visual FoxPro wants a name - an alias, a cursor, a column, a variable - a
* program may write an expression in parentheses instead, and the string it comes to is the
* name. A quoted name is a name too. The foundation classes lean on this everywhere.
LOCAL lcAlias, lcField, lcVar, lcCursor, lcColumn
lcAlias = "people"
lcField = "town"
lcVar = "lcMade"
lcCursor = "answer"
lcColumn = "county"

CREATE CURSOR (lcAlias) (name c(10), (lcField) c(10))
INSERT INTO (lcAlias) (name, (lcField)) VALUES ("ada", "leeds")
INSERT INTO (lcAlias) ("name", "town") VALUES ("bob", "hull")
? ALIAS(), FIELD(1), FIELD(2), RECCOUNT()

* a second cursor, so the IN clauses below have somewhere else to work on
CREATE CURSOR places (town c(10))
INSERT INTO places VALUES ("york")

* the work area an IN clause names is worked out the same way, and the one that is selected is
* left alone: REPLACE, APPEND BLANK and UNLOCK all take it
SELECT places
GO TOP IN (lcAlias)
REPLACE town WITH "derby" IN (lcAlias)
APPEND BLANK IN (lcAlias)
UNLOCK RECORD 1 IN (lcAlias)
? ALIAS(), RECCOUNT("people"), RECNO("people")

* a quoted name says the same thing as a written one, and REPLACE with nothing else to say
* changes the record the area is on, which the APPEND BLANK just made
REPLACE town WITH "wells" IN "people"
SELECT (lcAlias)
GO TOP
? ALLTRIM(town), ALLTRIM(people.name)
GO BOTTOM
? ALLTRIM(town)

* the cursor a query is gathered into, and the table it reads from
SELECT * FROM (lcAlias) WHERE NOT EMPTY(town) INTO CURSOR (lcCursor)
? ALIAS(), RECCOUNT()
SELECT * FROM "people" INTO CURSOR "written"
? ALIAS(), RECCOUNT()

* a column of a table being made, and one added to it afterwards
CREATE TABLE ("shire.dbf") FREE ((lcField) c(10))
ALTER TABLE ("shire.dbf") ADD COLUMN (lcColumn) c(8)
? ALIAS(), FIELD(1), FIELD(2)
USE

* a variable declared by a name it worked out, and one stored to the same way
PRIVATE (lcVar)
STORE 42 TO (lcVar)
? TYPE("lcMade"), lcMade

* what the name comes to may be a path through objects, not only a variable: this is how the
* foundation classes save and restore a property whose name they were handed
LOCAL loBox
loBox = CREATEOBJECT("Custom")
loBox.AddProperty("Caption", "before")
STORE "after" TO ("loBox." + "Caption")
? loBox.Caption
WITH loBox
   STORE "third" TO ("." + "Caption")
ENDWITH
? loBox.Caption

* and an array element is a name like any other
LOCAL aSlots(2)
STORE "kept" TO ("aSlots[2]")
? aSlots(2)

* COVERS: ALTER TABLE - SQL, BLANK, CREATE CURSOR - SQL, CREATE TABLE - SQL, INSERT - SQL, PRIVATE, REPLACE, SELECT - SQL, STORE, UNLOCK
