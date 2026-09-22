* ACOPY(): nFirstSourceElement, nNumberElements and nFirstDestElement.
DIMENSION aSrc(5), aDst1(5), aDst2(5), aDst3(5)
aSrc(1) = 10
aSrc(2) = 20
aSrc(3) = 30
aSrc(4) = 40
aSrc(5) = 50
? "ACOPY from the 3rd source element on", ACOPY(aSrc, aDst1, 3)
? aDst1(1), aDst1(2), aDst1(3), aDst1(4), aDst1(5)
? "ACOPY two elements starting at the 2nd", ACOPY(aSrc, aDst2, 2, 2)
? aDst2(1), aDst2(2), aDst2(3), aDst2(4), aDst2(5)
? "ACOPY into the destination's 4th element", ACOPY(aSrc, aDst3, 1, 2, 4)
? aDst3(1), aDst3(2), aDst3(3), aDst3(4), aDst3(5)

* ADEL()/AINS() with the flag that means a column rather than a row.
DIMENSION aGrid1(2,3), aGrid2(2,3)
aGrid1(1,1) = 1
aGrid1(1,2) = 2
aGrid1(1,3) = 3
aGrid1(2,1) = 4
aGrid1(2,2) = 5
aGrid1(2,3) = 6
? "ADEL column 2", ADEL(aGrid1, 2, 2)
? aGrid1(1,1), aGrid1(1,2), aGrid1(1,3)
? aGrid1(2,1), aGrid1(2,2), aGrid1(2,3)
aGrid2(1,1) = 1
aGrid2(1,2) = 2
aGrid2(1,3) = 3
aGrid2(2,1) = 4
aGrid2(2,2) = 5
aGrid2(2,3) = 6
? "AINS column 2", AINS(aGrid2, 2, 2)
? aGrid2(1,1), aGrid2(1,2), aGrid2(1,3)
? aGrid2(2,1), aGrid2(2,2), aGrid2(2,3)

* AELEMENT() with only a row given on a 2-D array is not "column missing" - the row is used
* directly as the element number, and it is checked against the row count, not the element
* count: a row past ALEN(a, 1) is error 1234 even though it would still be a valid element
* number taken linearly.
DIMENSION aTwo(2,3)
aTwo(1,1) = 11
? "AELEMENT row 1 alone", AELEMENT(aTwo, 1)
? "AELEMENT row 2 alone", AELEMENT(aTwo, 2)
TRY
	? AELEMENT(aTwo, 4)
CATCH TO oErr
	? "AELEMENT a row past the row count", oErr.ErrorNo
ENDTRY

* ALANGUAGE(): the reference page's syntax line never brackets nType (2-arg calls are already
* covered elsewhere), and the product means it - leaving nType out entirely is error 1229
* ("Too few arguments"), not nType defaulting to 1.
DIMENSION aLang2(1)
LOCAL nGot
TRY
	nGot = ALANGUAGE(aLang2)
CATCH TO oErr
	? "ALANGUAGE with no nType at all", oErr.ErrorNo
ENDTRY

* ASORT(): nStartElement, nNumberSorted, nSortOrder and nFlags together - nFlags 1 is a
* case-insensitive sort, and it does not merely treat differently-cased matches as equal and
* leave a stable sort to decide their order: measured, it breaks the tie itself.
DIMENSION aNums(5), aNums2(5), aWords(4), aNums3(3)
aNums(1) = 5
aNums(2) = 3
aNums(3) = 1
aNums(4) = 4
aNums(5) = 2
? "ASORT the middle three, ascending", ASORT(aNums, 2, 3)
? aNums(1), aNums(2), aNums(3), aNums(4), aNums(5)
aNums3(1) = 3
aNums3(2) = 1
aNums3(3) = 2
? "ASORT from the 2nd element on, no count given", ASORT(aNums3, 2)
? aNums3(1), aNums3(2), aNums3(3)
aNums2(1) = 5
aNums2(2) = 3
aNums2(3) = 1
aNums2(4) = 4
aNums2(5) = 2
? "ASORT all of it, descending", ASORT(aNums2, 1, -1, 1)
? aNums2(1), aNums2(2), aNums2(3), aNums2(4), aNums2(5)
aWords(1) = "banana"
aWords(2) = "Apple"
aWords(3) = "cherry"
aWords(4) = "apple"
? "ASORT case-insensitive", ASORT(aWords, 1, -1, 0, 1)
? aWords(1), aWords(2), aWords(3), aWords(4)

* ASELOBJ()/ASQLHANDLES(): nothing is selected in a designer here, and no connection is open,
* so the extra argument each takes does not need either one to actually exist.
DIMENSION aPicked(1), aHandles(1)
? "ASELOBJ with its container flag, nothing selected", ASELOBJ(aPicked, 1)
? "ASQLHANDLES with no connections open", ASQLHANDLES(aHandles)
TRY
	? ASQLHANDLES(aHandles, 1)
CATCH TO oErr
	? "ASQLHANDLES asked about a handle that does not exist", oErr.ErrorNo
ENDTRY

* APROCINFO() with no nType at all: the combined outline, naming a #DEFINE by its symbol and a
* class by its whole "Name AS Base" clause - not the same row shape kind 3 alone gives a define,
* nor the class name on its own.
LOCAL cSrc
cSrc = "#DEFINE MAX 10" + CHR(13) + CHR(10) + "DEFINE CLASS Thing AS Custom" + CHR(13) + CHR(10) + ;
	"PROCEDURE Init" + CHR(13) + CHR(10) + "ENDDEFINE"
= STRTOFILE(cSrc, "outline.prg")
DIMENSION aWhat(1)
? "APROCINFO with no nType", APROCINFO(aWhat, "outline.prg")
? aWhat(1,1), aWhat(1,3)
? aWhat(2,1), aWhat(2,3)
? aWhat(3,1), aWhat(3,3)

* AGETCLASS(), AMOUSEOBJ() and ANETRESOURCES() open a dialog, wait for a mouse over a design
* surface, or enumerate a network - nothing this project can ask about headlessly.

* COVERS: ACOPY, ADEL, AELEMENT, AINS, ALANGUAGE, APROCINFO, ASELOBJ, ASORT, ASQLHANDLES
