* The string functions of the language reference, one line each.
? ALLTRIM("  padded  ") + "|"
? LTRIM("  left") + "|"
? RTRIM("right  ") + "|"
? TRIM("both  ") + "|"
? UPPER("MiXeD"), LOWER("MiXeD"), PROPER("mixed case words")
? LEN("abcdef"), LEN("")
? LEFT("abcdef", 3), RIGHT("abcdef", 3), SUBSTR("abcdef", 2, 3), SUBSTR("abcdef", 4)
? AT("cd", "abcdef"), ATC("CD", "abcdef"), AT("z", "abcdef")
? RAT("a", "banana"), RATC("A", "banana")
? OCCURS("a", "banana"), OCCURS("z", "banana")
? PADL("7", 3, "0"), PADR("x", 3, "."), PADC("m", 5, "-")
? SPACE(3) + "|", REPLICATE("ab", 3)
? STRTRAN("a-b-c", "-", "+"), STRTRAN("aaaa", "a", "b", 2, 2)
? STUFF("abcdef", 2, 3, "XY")
? CHRTRAN("abcdef", "bd", "12")
? CHR(65), ASC("A")
? STR(42), STR(3.14159, 8, 3), ALLTRIM(STR(7))
? VAL("42.5"), VAL("12abc"), VAL("abc")
? TRANSFORM(1234.5, "999,999.99")
? GETWORDCOUNT("one two three"), GETWORDNUM("one two three", 2)
? STREXTRACT("[start]middle[end]", "[start]", "[end]")
? ISALPHA("abc"), ISDIGIT("123"), ISLOWER("abc"), ISUPPER("ABC")
? EMPTY(""), EMPTY("x"), ISBLANK("")
? INLIST("b", "a", "b", "c"), INLIST("z", "a", "b")
? BETWEEN(5, 1, 10), BETWEEN(0, 1, 10)
LOCAL aParts(1)
? ALINES(aParts, "one" + CHR(13) + "two"), aParts(1), aParts(2)
* COVERS: ALINES, ALLTRIM, ASC, AT, ATC, BETWEEN, CHR, CHRTRAN, EMPTY, GETWORDCOUNT, GETWORDNUM,
* COVERS: INLIST, ISALPHA, ISBLANK, ISDIGIT, ISLOWER, ISUPPER, LEFT, LEN, LOCAL, LOWER, LTRIM,
* COVERS: OCCURS, PADC, PADL, PADR, PROPER, RAT, RATC, REPLICATE, RIGHT, RTRIM, SPACE, STR,
* COVERS: STREXTRACT, STRTRAN, STUFF, SUBSTR, TRANSFORM, TRIM, UPPER, VAL
