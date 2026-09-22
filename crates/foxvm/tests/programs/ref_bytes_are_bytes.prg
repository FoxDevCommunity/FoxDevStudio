* COVERS: LEN, SUBSTR, AT, ASC, CHR
* A FoxPro string is bytes. A character above 127 is one byte, not two: a program packs a
* Windows structure into a string and cuts fields out of it by position, so a byte that
* counted as two would shift everything after it.
LOCAL lc
lc = "caf" + CHR(233)
? LEN(lc)
? AT(CHR(233), lc)
? ASC(SUBSTR(lc, 4, 1))
? LEN(CHR(200) + CHR(210) + CHR(7))
? ASC(SUBSTR(CHR(200) + CHR(210) + CHR(7), 2, 1))
? LEN(REPLICATE(CHR(255), 8))
? LEN(BINTOC(2002, "2RS"))
? ASC(LEFT(BINTOC(2002, "2RS"), 1))
? LEN(STRTRAN(CHR(233) + "a", "a", "bb"))
? LEN(PADR(CHR(233), 4))
