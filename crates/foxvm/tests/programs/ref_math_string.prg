* Wave 1 of the language reference: the functions that need nothing outside the VM.
SET DECIMALS TO 4
? PI()
? SIN(0), COS(0), TAN(0)
? ROUND(SIN(PI() / 2), 4), ROUND(COS(PI()), 4)
? ROUND(ASIN(1), 4), ROUND(ACOS(1), 4), ROUND(ATAN(1), 4)
? ROUND(ATN2(1, 1), 4), ROUND(ATN2(0, -1), 4)
? DTOR(180) = PI(), RTOD(PI()) = 180
SET DECIMALS TO 2
? BITSET(0, 0), BITSET(0, 3), BITSET(5, 1)
? BITCLEAR(7, 0), BITCLEAR(7, 1, 2)
? BITTEST(5, 0), BITTEST(5, 1), BITTEST(5, 2)
? NTOM(1.23456), NTOM(2), MTON(1.2345)
* the double-byte variants are the plain ones on a single-byte code page
? LENC("abc"), LEFTC("abcdef", 2), RIGHTC("abcdef", 2), SUBSTRC("abcdef", 2, 3)
? AT_C("cd", "abcdef"), ATCC("CD", "abcdef"), STUFFC("abcdef", 2, 1, "X")
? CHRTRANC("abc", "b", "B")
* phonetics
? SOUNDEX("Robert"), SOUNDEX("Rupert"), SOUNDEX("Smith"), SOUNDEX("Smyth")
? DIFFERENCE("Robert", "Rupert"), DIFFERENCE("Smith", "Smyth"), DIFFERENCE("Green", "Blue")
* patterns
? LIKE("a*e", "apple"), LIKE("a?e", "ape"), LIKE("a?e", "apple"), LIKE("*", "anything")
? LIKE("A*", "apple")
* memo lines, at the width they wrap to
SET MEMOWIDTH TO 20
LOCAL lcMemo
lcMemo = "the quick brown fox jumps over the lazy dog"
? MEMLINES(lcMemo)
? MLINE(lcMemo, 1)
? MLINE(lcMemo, 2)
? ATLINE("jumps", lcMemo), ATCLINE("JUMPS", lcMemo), ATLINE("nowhere", lcMemo)
? RATLINE("the", lcMemo)
SET MEMOWIDTH TO 50
? MEMLINES(lcMemo)
* conversions
? STRCONV("Mixed Case", 7), STRCONV("Mixed Case", 8)
? STRCONV("abc", 13)
? STRCONV("YWJj", 14)
? STRCONV("abc", 15)
? STRCONV("616263", 16)
? LEN(STRCONV("ab", 5))
? STRCONV(STRCONV("round trip", 13), 14)
* COVERS: ACOS, ASIN, ATAN, ATCC, ATCLINE, ATLINE, ATN2, AT_C, BITCLEAR, BITSET, BITTEST,
* COVERS: CHRTRANC, COS, DIFFERENCE, DTOR, LEFTC, LEN, LENC, LIKE, LOCAL, MEMLINES, MLINE, MTON,
* COVERS: NTOM, PI, RATLINE, RIGHTC, ROUND, RTOD, SET DECIMALS, SET MEMOWIDTH, SIN, SOUNDEX,
* COVERS: STRCONV, STUFFC, SUBSTRC, TAN
