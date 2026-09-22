* The buffering functions asked about a work area that is not the current one, and with the
* forms that set rather than only read: CURSORSETPROP's own value and area arguments, and
* TABLEUPDATE/TABLEREVERT with no argument at all.
ON ERROR ?? ""
SET SAFETY OFF
* table buffering (CURSORSETPROP's mode 5) needs SET MULTILOCKS ON, or it is refused
SET MULTILOCKS ON

CREATE TABLE staff (iID i AUTOINC NEXTVALUE 100 STEP 1, name C(10), pay N(8,2))
INSERT INTO staff (name, pay) VALUES ("Nolan", 3200)
INSERT INTO staff (name, pay) VALUES ("Ames", 4100)

SELECT 2
CREATE TABLE dept (code C(4))
INSERT INTO dept (code) VALUES ("S1")

SELECT 1

* --- CURSORSETPROP(): the property alone, defaulting its value; and property, value and area
? CURSORSETPROP("Buffering")
? CURSORSETPROP("Buffering", 5, 2)
? CURSORGETPROP("Buffering", 2)
? CURSORSETPROP("Buffering", 5)

GO TOP
REPLACE name WITH "Nolan Jr"
SELECT 2
GO TOP
REPLACE code WITH "S2"
SELECT 1

* --- CURVAL(), OLDVAL(): asked about a table elsewhere
? CURVAL("code", 2), OLDVAL("code", 2)

* --- GETFLDSTATE(): asked about a table elsewhere
? GETFLDSTATE(1, 2)

* --- SETFLDSTATE(): field, state and area together
? SETFLDSTATE(1, 2, 2), GETFLDSTATE(1, 2)

* --- GETNEXTMODIFIED(): asked about a table elsewhere
? GETNEXTMODIFIED(0, 2)

* --- TABLEREVERT(): no argument at all, and asked about a table elsewhere
? TABLEREVERT()
? ALLTRIM(name)
? TABLEREVERT(.T., 2)
? ALLTRIM(dept.code)

* --- TABLEUPDATE(): no argument, then row count and force together, then an alias too
REPLACE name WITH "Nolan Jr"
? TABLEUPDATE()
? TABLEUPDATE(1, .T.)
REPLACE name WITH "Nolan Sr"
? TABLEUPDATE(1, .T., "staff")
USE staff
GO TOP
? ALLTRIM(name)

* --- GETAUTOINCVALUE(): the current data session, by number
? GETAUTOINCVALUE(1)

SELECT 2
USE
SELECT 1
USE

* COVERS: CURSORGETPROP, CURSORSETPROP, CURVAL, GETAUTOINCVALUE, GETFLDSTATE, GETNEXTMODIFIED, OLDVAL, SETFLDSTATE, TABLEREVERT, TABLEUPDATE
