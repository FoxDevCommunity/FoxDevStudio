* COVERS: BINTOC, CTOBIN
* Every flag the two of them take. A program reaching the Windows API packs a struct with
* BINTOC and reads the answer back with CTOBIN, so the size, the sign and the byte order have
* to be exactly what the product does: a SYSTEMTIME is eight little-endian words, a file size
* is a little-endian eight-byte number, and reading either the wrong way round gives a number
* that looks like nothing at all.
LOCAL lcFour, lcTwo
* what a little-endian word and long look like as characters
lcTwo = CHR(210) + CHR(7)
lcFour = CHR(0) + CHR(0) + CHR(0) + CHR(1)
? CTOBIN(lcTwo, "2RS")
? CTOBIN(lcTwo, "2R")
? CTOBIN(lcTwo, "2S")
? CTOBIN(lcTwo, "2")
? CTOBIN(lcFour, "4RS")
? CTOBIN(lcFour, "4R")
? CTOBIN(lcFour, "4S")
? CTOBIN(lcFour, "4")
? CTOBIN(CHR(255) + CHR(255), "2RS")
? CTOBIN(CHR(255) + CHR(255), "2R")
* and the same numbers packed again
? LEN(BINTOC(2002, "2RS"))
? CTOBIN(BINTOC(2002, "2RS"), "2RS")
? CTOBIN(BINTOC(-5, "2RS"), "2RS")
? CTOBIN(BINTOC(64, "4RS"), "4RS")
? CTOBIN(BINTOC(1000000000, "8RS"), "8RS")
? CTOBIN(BINTOC(2002, "2"), "2")
? CTOBIN(BINTOC(70000, "4"), "4")
? ASC(SUBSTR(BINTOC(1, "4RS"), 1, 1))
? ASC(SUBSTR(BINTOC(1, "4"), 1, 1))
? ASC(SUBSTR(BINTOC(1, "4S"), 1, 1))
