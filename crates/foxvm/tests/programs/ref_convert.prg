* The functions that convert, choose and merge.
? BINTOC(258) == CHR(128) + CHR(0) + CHR(1) + CHR(2)
? BINTOC(258, 2) == CHR(129) + CHR(2)
? BINTOC(-1, 1) == CHR(127)
? BINTOC(258, "S") == CHR(0) + CHR(0) + CHR(1) + CHR(2)
? BINTOC(258, "R") == CHR(2) + CHR(1) + CHR(0) + CHR(128)
? CTOBIN(BINTOC(258)), CTOBIN(BINTOC(-5, 2), "2")
? CTOBIN(BINTOC(1.5, 8), "B")
? LEN(CREATEBINARY("abc"))

* the condition that chooses, and the one that does not
? ICASE(.F., 1, .T., 2, 3)
? ICASE(.F., 1, .F., 2, 9)
? ISNULL(ICASE(.F., 1, .F., 2))

* text with expressions in it
gnX = 7
? TEXTMERGE("a<<gnX>>b")
? TEXTMERGE("a{{gnX}}b", .F., "{{", "}}")

* an expression written the one way Visual FoxPro writes it
? NORMALIZE("upper(a) and b")
? NORMALIZE("cust->name = [Hello]")
? NORMALIZE("alltr( name ) + [ x ]")
? NORMALIZE("a .and. not b or c")

* what stands for a name, and how the pattern reads
? LIKEC("a*", "abc"), LIKE("a*", "abc")
? EMPTY(GETENV("NO_SUCH_THING_AT_ALL"))
? GETCP(1252), GETCP()

CREATE TABLE marked (a C(3))
INSERT INTO marked VALUES ("one")
? CPDBF() > 0
? REFRESH(1)
USE
DROP TABLE marked.dbf

* COVERS: BINTOC, CPDBF, CREATEBINARY, CTOBIN, GETCP, GETENV, ICASE, LIKEC, NORMALIZE,
* COVERS: REFRESH, TEXTMERGE
