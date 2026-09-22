* Shapes the Visual FoxPro samples and Foundation Classes are written in.
* an object whose name abbreviates a command
LOCAL loCat, lcName, lcSet
loCat = CREATEOBJECT("Empty")
ADDPROPERTY(loCat, "Connected", .F.)
loCat.Connected = .T.
? loCat.Connected
* CAST with the type after the value
? CAST("42" AS I) + 1
? CAST(7 AS C)
? ISNULL(CAST(NULL AS I))
* output placed at a column, which has none here
? "at" AT 10
* a line that begins with a macro
lcName = "lcSet"
&lcName = "assigned through a macro"
? lcSet
&lcName. = "with the dot too"
? lcSet
* text clauses in any order
TEXT TO lcSet TEXTMERGE NOSHOW ADDITIVE
 more
ENDTEXT
? lcSet
SET TEXTMERGE ON NOSHOW
SET TEXTMERGE TO MEMVAR lcSet NOSHOW
* ERASE with the recycle bin named
STRTOFILE("x", "gone.txt")
ERASE ("gone.txt") NORECYCLE
? FILE("gone.txt")
* COVERS: ADDPROPERTY, CAST, CREATEOBJECT, ERASE, FILE, IF ... ENDIF, ISNULL, LOCAL,
* COVERS: SET TEXTMERGE, STRTOFILE, TEXT ... ENDTEXT
