* What a memo field holds, between a file and a window on it.
CREATE TABLE notes (name C(8), body M)
APPEND BLANK
REPLACE name WITH "first", body WITH "the first line"

* out to a file of its own, and back again beside what is there
COPY MEMO body TO note.txt
APPEND MEMO body FROM note.txt
? LEN(ALLTRIM(body)), LEFT(body, 14)

* and over what is there
APPEND MEMO body FROM note.txt OVERWRITE
? LEN(ALLTRIM(body))

* a window on it: the mock host hands back what it was shown
MODIFY MEMO body
? ALLTRIM(body)
MODIFY MEMO body NOWAIT
CLOSE MEMO ALL

* APPEND on its own adds a record to write into
APPEND
? RECCOUNT(), EMPTY(name)

USE
DROP TABLE notes.dbf

* COVERS: APPEND, APPEND MEMO, CLOSE MEMO, COPY MEMO, MODIFY MEMO
