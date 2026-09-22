* CREATEOFFLINE() with the path an offline view's records go to, and REFRESH() with no
* argument at all, and with the record count paired with an offset and then a work area - the
* forms nobody had measured, because they all need a SQL view with an updatable key to ask.
ON ERROR ?? ""
SET SAFETY OFF

CREATE DATABASE shop
CREATE TABLE stock (code C(4), name C(12))
INSERT INTO stock VALUES ("A1", "Bolt")
INSERT INTO stock VALUES ("B2", "Nut")
USE
CREATE SQL VIEW cheap AS SELECT * FROM stock
USE cheap
? CURSORSETPROP("KeyFieldList", "code")

* --- REFRESH(): no argument at all
? REFRESH()

* --- REFRESH(nRecords, nRecordOffset): the record count paired with an offset
? REFRESH(1, 0)

* --- REFRESH(nRecords, nRecordOffset, area): a work area named alongside them
SELECT 2
? REFRESH(1, 0, "cheap")
SELECT 1

* --- CREATEOFFLINE(cViewName, cPath): where the offline copy's records go
? CREATEOFFLINE("cheap", CURDIR() + "cheapoff")

USE
CLOSE DATABASES
DELETE DATABASE shop.dbc DELETETABLES

* COVERS: CREATEOFFLINE, REFRESH
