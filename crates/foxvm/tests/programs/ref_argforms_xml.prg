* COVERS: CURSORTOXML, XMLTOCURSOR, XMLUPDATEGRAM
* The XML functions' forms no other golden calls: CURSORTOXML with its output format, its flags,
* a record limit and a schema name; XMLTOCURSOR reading a document straight from a string, into
* a cursor of its own choosing, and told to preserve whitespace; XMLUPDATEGRAM with no argument
* at all, an alias, flags, and a schema location.

* SET MULTILOCKS has to be ON before any table or cursor is opened, so it comes first - it is
* what XMLUPDATEGRAM's buffered changes need. TABLEUPDATE() commits the two inserts, so REPLACE
* later has a stored value of its own to report as "before" - a row still new since it was
* buffered has no such thing to show.
SET MULTILOCKS ON
CREATE CURSOR xmlsrc (name C(10), amt N(6,2))
= CURSORSETPROP("Buffering", 5)
INSERT INTO xmlsrc VALUES ("Alice", 12.50)
INSERT INTO xmlsrc VALUES ("Bob", 7.25)
= TABLEUPDATE(.T.)

* --- CURSORTOXML(2): writes into the memory variable cOutput names, creating it, and answers
* with the byte count - not the XML itself
n2 = CURSORTOXML("xmlsrc", "vOut2")
? "CURSORTOXML(2) count", n2 == LEN(vOut2)
? vOut2

* --- CURSORTOXML(3): nOutputFormat 2 writes each field as an attribute
= CURSORTOXML("xmlsrc", "vOut3", 2)
? "CURSORTOXML(3) attributes", vOut3

* --- CURSORTOXML(4): nFlags 1 is a continuous string, with no line breaks at all
= CURSORTOXML("xmlsrc", "vOut4", 1, 1)
? "CURSORTOXML(4) continuous", vOut4

* --- CURSORTOXML(5): nRecords greater than 0 exports that many records starting from the record
* pointer, rather than the whole table from the top
GO TOP
SKIP
= CURSORTOXML("xmlsrc", "vOut5", 1, 1, 1)
? "CURSORTOXML(5) one record from the pointer", vOut5

* --- CURSORTOXML(6): cSchemaName, a file name rather than "1", names an external schema in an
* xsi:noNamespaceSchemaLocation attribute rather than writing one inline
= CURSORTOXML("xmlsrc", "vOut6", 1, 1, 0, "myschema.xsd")
? "CURSORTOXML(6) names the schema", 'xsi:noNamespaceSchemaLocation="myschema.xsd"' $ vOut6
? "CURSORTOXML(6) still has both rows", OCCURS("<xmlsrc>", vOut6)

* --- XMLTOCURSOR(1): the source alone, naming no cursor - it lands in XMLRESULT
= XMLTOCURSOR(vOut2)
? "XMLTOCURSOR(1) default alias", ALIAS(), RECCOUNT()

* --- XMLTOCURSOR(2): a cursor name of its own
= XMLTOCURSOR(vOut2, "roundtrip")
? "XMLTOCURSOR(2) chosen alias", ALIAS(), RECCOUNT()
GO TOP
? "XMLTOCURSOR(2) reads it back", ALLTRIM(name), amt

* --- XMLTOCURSOR(3): nFlags 4 preserves whitespace in a field, rather than trimming it
= XMLTOCURSOR([<VFPData><row name="  padded  "/></VFPData>], "spaced", 4)
? "XMLTOCURSOR(3) keeps the spaces", "[" + name + "]"

* --- XMLUPDATEGRAM(0): no argument uses the current work area
SELECT xmlsrc
GO TOP
REPLACE amt WITH 99.00
g0 = XMLUPDATEGRAM()
? "XMLUPDATEGRAM(0) one sync", OCCURS("updg:sync", g0) / 2
? "XMLUPDATEGRAM(0) before and after", IIF("12.50" $ g0, "before", "no before"), IIF("99.00" $ g0, "after", "no after")

* --- XMLUPDATEGRAM(1): the alias named explicitly
g1 = XMLUPDATEGRAM("xmlsrc")
? "XMLUPDATEGRAM(1) same as (0)", g1 == g0

* --- XMLUPDATEGRAM(2): nFlags 1 is a continuous string, the same as CURSORTOXML's
g2 = XMLUPDATEGRAM("xmlsrc", 1)
? "XMLUPDATEGRAM(2) continuous", NOT (CHR(13) $ g2)

* --- XMLUPDATEGRAM(3): cSchemaLocation names the mapping schema on the sync element
g3 = XMLUPDATEGRAM("xmlsrc", 1, "myschema.xsd")
? "XMLUPDATEGRAM(3) names the schema", 'mapping-schema="myschema.xsd"' $ g3
