* COVERS: REPLACE
*
* ADDITIVE adds to a memo field rather than writing over it. Every answer below was measured in
* Visual FoxPro 9.
CREATE CURSOR notes (id N(2), body M, tail M, label C(6))
INSERT INTO notes (id, body, tail, label) VALUES (1, "", "", "")
INSERT INTO notes (id, body, tail, label) VALUES (2, "xy", "pq", "xy")
INSERT INTO notes (id, body, tail, label) VALUES (3, "xy   ", "pq", "xy")

* an empty memo takes the text as it is; one with something in it keeps what it had
GO 1
REPLACE body WITH "abc" ADDITIVE
? "empty:", "[" + body + "]"
GO 2
REPLACE body WITH "abc" ADDITIVE
? "not empty:", "[" + body + "]"

* neither side is trimmed: the blanks a memo ends in are part of it
GO 3
REPLACE body WITH "abc" ADDITIVE
? "trailing blanks:", "[" + body + "]", LEN(body)

* a field that is not a memo is written over, and no complaint is made about the word
GO 2
REPLACE label WITH "abc" ADDITIVE
? "character:", "[" + label + "]"

* the value is worked out before anything is added, so a memo may be added to itself
GO 2
REPLACE body WITH body + "!" ADDITIVE
? "itself:", "[" + body + "]"

* adding nothing leaves the memo as it was
REPLACE body WITH "" ADDITIVE
? "nothing:", "[" + body + "]"

* ADDITIVE belongs to the assignment it is written after and not to the command
GO 1
REPLACE body WITH "Q", tail WITH "R" ADDITIVE
? "second only:", "[" + body + "]", "[" + tail + "]"
GO 1
REPLACE body WITH "S" ADDITIVE, tail WITH "T"
? "first only:", "[" + body + "]", "[" + tail + "]"

* a scope adds to each record in it, one at a time
REPLACE ALL tail WITH "|z" ADDITIVE
DO show WITH "all of them"
REPLACE ALL tail WITH "|w" ADDITIVE FOR id > 1
DO show WITH "some of them"

* the field the name works out to, added to the same way
LOCAL cField
cField = "body"
GO 2
REPLACE (cField) WITH "-named" ADDITIVE
? "by name:", "[" + body + "]"

* a memo in another work area, reached by the name of its field
SELECT notes
GO 1
REPLACE notes.body WITH "+far" ADDITIVE
? "qualified:", "[" + notes.body + "]"

* what is added has to be text, and the memo is left alone when it is not
TRY
   GO 2
   REPLACE body WITH 5 ADDITIVE
CATCH TO oErr
   ? "a number:", oErr.ErrorNo, oErr.Message
ENDTRY
? "after that:", "[" + body + "]"

* every scope a REPLACE takes, each adding to the records it reaches and leaving the rest
CREATE CURSOR scoped (id N(2), body M)
INSERT INTO scoped VALUES (1, "a")
INSERT INTO scoped VALUES (2, "b")
INSERT INTO scoped VALUES (3, "c")
INSERT INTO scoped VALUES (4, "d")
GO 2
REPLACE NEXT 2 body WITH "+n" ADDITIVE
DO scope WITH "next two from the second"
GO 1
REPLACE RECORD 3 body WITH "+r" ADDITIVE
DO scope WITH "record three"
GO 3
REPLACE REST body WITH "+s" ADDITIVE
DO scope WITH "the rest from the third"
GO TOP
REPLACE ALL body WITH "+f" ADDITIVE FOR id = 2 OR id = 4
DO scope WITH "for the even ones"
GO TOP
REPLACE REST body WITH "+w" ADDITIVE WHILE id < 3
DO scope WITH "while the id is under three"
GO 2
REPLACE body WITH "+c" ADDITIVE
DO scope WITH "this record and no other"

USE IN notes
USE IN scoped
RETURN

PROCEDURE scope(cLabel)
? cLabel
SELECT scoped
SCAN
   ? "  ", id, "[" + body + "]"
ENDSCAN

PROCEDURE show(cLabel)
? cLabel
SELECT notes
SCAN
   ? "  ", id, "[" + tail + "]"
ENDSCAN
