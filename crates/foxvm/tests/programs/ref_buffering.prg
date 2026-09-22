* Wave 5 of the language reference: buffering and transactions.
CREATE TABLE ledger (payee C(10), amount N(8,2))
INSERT INTO ledger (payee, amount) VALUES ("Ames", 100)
INSERT INTO ledger (payee, amount) VALUES ("Bell", 200)
? CURSORGETPROP("Buffering")
? CURSORSETPROP("Buffering", 5), CURSORGETPROP("Buffering")
? CURSORGETPROP("Alias")
* a change the program can see and the file has not got yet
GO TOP
REPLACE payee WITH "Ames Ltd"
? ALLTRIM(payee), GETFLDSTATE(1), GETFLDSTATE(2), GETFLDSTATE("payee")
? GETNEXTMODIFIED(0)
* and one that is thrown away instead
? TABLEREVERT(.T.), ALLTRIM(payee), GETFLDSTATE(1)
REPLACE payee WITH "Ames Ltd"
GO BOTTOM
REPLACE amount WITH 250
? GETNEXTMODIFIED(0), GETNEXTMODIFIED(1), GETNEXTMODIFIED(2)
? SETFLDSTATE(2, 2), GETFLDSTATE(2)
? TABLEUPDATE(.T.)
? GETNEXTMODIFIED(0)
* what is written is what the file holds
USE ledger
GO TOP
? ALLTRIM(payee)
GO BOTTOM
? amount
* a record appended while the table is buffered, and taken back again
? CURSORSETPROP("Buffering", 5)
APPEND BLANK
REPLACE payee WITH "Cirrus"
? RECCOUNT(), GETFLDSTATE(1)
? TABLEREVERT(.T.), RECCOUNT()
* a transaction holds every write until it ends
? TXNLEVEL()
BEGIN TRANSACTION
? TXNLEVEL()
GO TOP
REPLACE amount WITH 999
END TRANSACTION
? TXNLEVEL()
USE ledger
GO TOP
? amount
* and one that is rolled back changes nothing
BEGIN TRANSACTION
REPLACE amount WITH 1
ROLLBACK
? TXNLEVEL(), amount
USE
DROP TABLE ledger.dbf
* COVERS: APPEND, BEGIN TRANSACTION, CREATE TABLE, CURSORGETPROP, CURSORSETPROP, DROP TABLE,
* COVERS: END TRANSACTION, GETFLDSTATE, GETNEXTMODIFIED, GO, INSERT, RECCOUNT, REPLACE,
* COVERS: ROLLBACK, SETFLDSTATE, TABLEREVERT, TABLEUPDATE, TXNLEVEL, USE
