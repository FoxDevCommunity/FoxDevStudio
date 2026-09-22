* The memory variables a program puts away and takes back out again.
PUBLIC gcWho, gnHowMany
gcWho = "Jorge"
gnHowMany = 42.75
lReady = .T.
dWhen = {^2026-09-08}
DIMENSION aList(3)
aList(1) = "one"
aList(2) = 2
aList(3) = .F.

SAVE TO work.mem
RELEASE ALL
? TYPE("gcWho")
RESTORE FROM work.mem
? gcWho, gnHowMany, lReady, DTOS(dWhen)
? ALEN(aList), aList(1), aList(2), aList(3)

* what is in memory already stays when the command says so
cKept = "kept"
RESTORE FROM work.mem ADDITIVE
? cKept, gcWho

* only the names the skeleton keeps go in the file
SAVE TO some.mem ALL LIKE g*
RELEASE ALL
RESTORE FROM some.mem
? TYPE("gcWho"), TYPE("lReady")

* and the other way round
gcWho = "Jorge"
lReady = .T.
SAVE TO other.mem ALL EXCEPT g*
RELEASE ALL
RESTORE FROM other.mem
? TYPE("gcWho"), TYPE("lReady")

* a memo field holds the same thing a file does
CREATE TABLE notes (name C(8), vars M)
APPEND BLANK
REPLACE name WITH "first"
cSaved = "in the memo"
SAVE TO MEMO vars ALL LIKE cSaved
cSaved = "changed"
RESTORE FROM MEMO vars ADDITIVE
? cSaved
USE
DROP TABLE notes.dbf

* COVERS: RESTORE FROM, SAVE TO
