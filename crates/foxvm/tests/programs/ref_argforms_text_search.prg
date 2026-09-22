* AT(), ATC(), ATCC(), AT_C(), RAT() and RATC() with an explicit occurrence, and what an
* occurrence of 0 does: error 11, not the first occurrence a lenient clamp would give.
LOCAL lcHay
lcHay = "abracadabra"
? "AT 3rd occurrence", AT("a", lcHay, 3)
? "ATC case-insensitive, 2nd occurrence", ATC("A", lcHay, 2)
? "ATCC is ATC on a single-byte code page", ATCC("A", lcHay, 2)
? "AT_C is AT on a single-byte code page", AT_C("a", lcHay, 3)
? "RAT with no occurrence is the last one", RAT("a", lcHay)
? "RAT 2nd occurrence from the end", RAT("a", lcHay, 2)
? "RATC is case-sensitive, unlike its name's C", RATC("A", "banana", 1)
TRY
	? AT("a", lcHay, 0)
CATCH TO oErr
	? "AT occurrence 0 is error 11, not occurrence 1", oErr.ErrorNo
ENDTRY

* GETWORDCOUNT()/GETWORDNUM() with delimiters of their own: every character in cDelimiters is
* its own separator, the default ones stop applying once any are given, and a word next to a
* delimiter that is not itself a delimiter (the space after each comma here) is not trimmed.
LOCAL lcWords
lcWords = "AAA aaa, BBB bbb, CCC ccc."
? "GETWORDCOUNT with the default delimiters", GETWORDCOUNT(lcWords)
? "GETWORDCOUNT split only on commas", GETWORDCOUNT(lcWords, ",")
? "GETWORDNUM the 3rd word by default", GETWORDNUM(lcWords, 3)
? "GETWORDNUM by comma keeps the leading space", "[" + GETWORDNUM(lcWords, 2, ",") + "]"
? "GETWORDNUM past the last word is empty", "[" + GETWORDNUM(lcWords, 99) + "]"

* COVERS: AT, ATC, ATCC, AT_C, RAT, RATC, GETWORDCOUNT, GETWORDNUM
