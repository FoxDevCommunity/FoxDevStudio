* More of the language reference: money, dates, and what a table says about itself.
? PAYMENT(10000, 0.01, 12)
? ROUND(FV(100, 0.01, 12), 2)
? ROUND(PV(100, 0.01, 12), 2)
? QUARTER({^2026-02-14}), QUARTER({^2026-11-01})
SET CENTURY ON
? DMY({^2026-09-07})
? MDY({^2026-09-07})
SET CENTURY OFF
? DMY({^2026-09-07})
CREATE TABLE stock (code C(6), name C(10), price N(8,2), notes M)
INSERT INTO stock (code, name, price, notes) VALUES ("A1", "Bolt", 2.50, "zinc plated")
INSERT INTO stock (code, name, price, notes) VALUES ("B2", "Nut", 1.75, "brass")
? RECSIZE(), HEADER() > 0
? ISREADONLY(), ISEXCLUSIVE()
LOCAL aOpen(1)
? AUSED(aOpen), aOpen(1, 1), aOpen(1, 2)
INDEX ON code TAG bycode FOR price > 1
? FOR(1), IDXCOLLATE(1)
? KEYMATCH("A1"), KEYMATCH("ZZ")
? RECNO()
? ALLTRIM(LOOKUP(name, "B2", code, "bycode"))
? ALLTRIM(CURVAL("name"))
GO TOP
REPLACE name WITH "Bolt XL"
? ALLTRIM(name), UPDATED()
* a table with no autoincrementing field has generated no value at all, which is .NULL.
? GETAUTOINCVALUE(), ISMEMOFETCHED("notes")
? MDX(1) == "stock.CDX"
USE
DROP TABLE stock.dbf
* COVERS: AUSED, CURVAL, DMY, FOR, FV, GETAUTOINCVALUE, HEADER, IDXCOLLATE, ISEXCLUSIVE,
* COVERS: ISMEMOFETCHED, ISREADONLY, KEYMATCH, LOOKUP, MDX, MDY, PAYMENT, PV, QUARTER, RECSIZE,
* COVERS: UPDATED
