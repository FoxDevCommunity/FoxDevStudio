* System 4 of the parity plan: the single-entry .idx index beside a table.
CREATE TABLE people (name C(10), age N(3,0))
INSERT INTO people (name, age) VALUES ("delta", 40)
INSERT INTO people (name, age) VALUES ("alpha", 10)
INSERT INTO people (name, age) VALUES ("charlie", 30)
INSERT INTO people (name, age) VALUES ("bravo", 20)
INSERT INTO people (name, age) VALUES ("echo", 50)
* an index of its own rather than a tag: the older layout, and the compact one
INDEX ON name TO idxname
? ORDER(), KEY(1), NDX(1), NDX(2) == "", CDX(1) == ""
GO TOP
? ALLTRIM(name)
GO BOTTOM
? ALLTRIM(name), RECNO()
INDEX ON age TO idxage COMPACT
? ORDER(), TAGCOUNT()
GO TOP
? ALLTRIM(name), age
* seeking down one finds the record it belongs to
SEEK 30
? FOUND(), ALLTRIM(name), RECNO()
SEEK 99
? FOUND(), EOF()
* opened again with the table, the first of them controlling
USE
USE people INDEX idxname, idxage
? ORDER(), NDX(1), NDX(2), TAGCOUNT()
GO TOP
? ALLTRIM(name)
? SEEK("charlie"), RECNO()
SET ORDER TO 2
? ORDER(), SEEK(50), ALLTRIM(name)
* SET INDEX opens them and closes them
SET INDEX TO idxname
? ORDER(), NDX(1), NDX(2) == ""
GO TOP
? ALLTRIM(name)
SET INDEX TO
? ORDER() == "", NDX(1) == ""
GO TOP
? ALLTRIM(name)
* a tag of the compound index written out as an index of its own
INDEX ON UPPER(name) TAG upname
COPY TAG upname TO idxupper
SET INDEX TO idxupper
? ORDER(), KEY(1), NDX(1)
GO TOP
? ALLTRIM(name)
? SEEK("ECHO"), RECNO()
* and single-entry indexes turned back into tags, in a compound index of their own
SET INDEX TO idxname, idxage
COPY INDEXES idxname, idxage TO made.cdx
? FILE("made.cdx")
* and in the structural one beside the table
COPY INDEXES ALL
USE
USE people
? TAGCOUNT(), TAG(1), TAG(2), TAG(3)
? KEY(TAGNO("IDXAGE")), NDX(1) == ""
SET ORDER TO IDXNAME
GO TOP
? ALLTRIM(name)
USE
* COVERS: ALLTRIM, COPY INDEXES, COPY TAG, CREATE TABLE, EOF, FILE, FOUND, GO, INDEX, INSERT,
* COVERS: KEY, NDX, ORDER, RECNO, SEEK, SET INDEX, SET ORDER, TAG, TAGCOUNT, TAGNO, UPPER, USE
