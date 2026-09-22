* PADL()/PADC() with the pad character - the third argument, past the space they already
* default to.
? "PADL default pad", "[" + PADL("hi", 6) + "]"
? "PADL a custom pad character", "[" + PADL("hi", 6, "*") + "]"
? "PADC default pad", "[" + PADC("hi", 6) + "]"
? "PADC a custom pad character", "[" + PADC("hi", 7, "-") + "]"

* SUBSTRC() is SUBSTR() on a single-byte code page.
? "SUBSTRC to the end", SUBSTRC("Hello World", 7)
? "SUBSTRC with a length", SUBSTRC("Hello World", 7, 3)

* STRTRAN(): nStartOccurrence and nNumberOfOccurrences pick which matches are replaced, and
* nFlags can case-adjust the replacement to what each match itself was - measured, one call
* answers a differently-cased replacement for a title-case match than for an upper-case one.
* An explicit 0 in either occurrence argument is error 11, the same as AT()'s own.
LOCAL lcTri
lcTri = "one two one two one two"
? "STRTRAN default replaces every one", STRTRAN(lcTri, "one", "X")
? "STRTRAN with no replacement removes it", "[" + STRTRAN(lcTri, "one") + "]"
? "STRTRAN from the 2nd occurrence onward", STRTRAN(lcTri, "one", "X", 2)
? "STRTRAN one occurrence starting at the 2nd", STRTRAN(lcTri, "one", "X", 2, 1)
? "STRTRAN flag 1 is case-insensitive, exact replacement", STRTRAN("One TWO one", "one", "x", 1, -1, 1)
? "STRTRAN flag 3 case-adjusts what flag 1 found", STRTRAN("One two ONE", "one", "x", 1, -1, 3)
TRY
	? STRTRAN(lcTri, "one", "X", 0)
CATCH TO oErr
	? "STRTRAN nStartOccurrence 0", oErr.ErrorNo
ENDTRY
TRY
	? STRTRAN(lcTri, "one", "X", 1, 0)
CATCH TO oErr
	? "STRTRAN nNumberOfOccurrences 0", oErr.ErrorNo
ENDTRY

* STREXTRACT(): nFlags bit value 2 is the end delimiter being optional, and bit value 4 is
* keeping the delimiters in the answer - the reverse of what this runtime had the two doing
* until it was measured.
LOCAL lcTag
lcTag = "<a>hello</a><b>world</b>"
? "STREXTRACT plain", STREXTRACT(lcTag, "<a>", "</a>")
? "STREXTRACT the 3rd occurrence of the begin delimiter", STREXTRACT(lcTag, "<", ">", 3)
? "STREXTRACT flag 1 is case-insensitive delimiters", STREXTRACT("<A>hi</A>", "<a>", "</a>", 1, 1)
? "STREXTRACT flag 2 is the missing end delimiter", STREXTRACT("<a>hello", "<a>", "</a>", 1, 2)
? "STREXTRACT flag 4 keeps the delimiters", STREXTRACT(lcTag, "<a>", "</a>", 1, 4)
? "STREXTRACT with no end delimiter at all reads to the end", STREXTRACT(lcTag, "<a>")

* STRCONV(): a regional identifier and its type are accepted alongside the conversion setting.
? "STRCONV uppercase, plain", STRCONV("hello", 8)
? "STRCONV uppercase, with a locale", STRCONV("hello", 8, 1033)
? "STRCONV uppercase, with a locale and its type", STRCONV("hello", 8, 1033, 0)

* TEXTMERGE(): lRecursive re-scans what the first pass wrote for more merge fields, and a left
* delimiter given alone keeps the default right delimiter rather than needing both.
LOCAL lcInner
lcInner = "<<1+1>>"
? "TEXTMERGE not recursive leaves the merged text alone", TEXTMERGE("value is <<lcInner>>", .F.)
? "TEXTMERGE recursive evaluates the merged text too", TEXTMERGE("value is <<lcInner>>", .T.)
? "TEXTMERGE a left delimiter alone keeps >> on the right", TEXTMERGE("value is [[1+1>>", .F., "[[")
? "TEXTMERGE both delimiters given", TEXTMERGE("value is [1+1]", .F., "[", "]")

* MLINE(): the third argument is a character offset counted from the start of the whole memo,
* not from the line nRowOption names - measured, it can walk MLINE() past the row asked for and
* onto a later one, returned whole rather than trimmed by whatever offset is left over.
LOCAL lcMemo
lcMemo = "one two three four five" + CHR(13) + CHR(10) + "second line here" + CHR(13) + CHR(10) + "third"
? "MLINE row 1 from its own start", MLINE(lcMemo, 1, 0)
? "MLINE row 1 skipping five characters", MLINE(lcMemo, 1, 5)
? "MLINE row 1 to exactly its own end is empty", "[" + MLINE(lcMemo, 1, 23) + "]"
? "MLINE row 1 one past its own end lands on row 2 whole", MLINE(lcMemo, 1, 24)
? "MLINE row 2 with an offset that has not reached it yet is row 2 whole", MLINE(lcMemo, 2, 0)

* COVERS: PADL, PADC, SUBSTRC, STRTRAN, STREXTRACT, STRCONV, TEXTMERGE, MLINE
