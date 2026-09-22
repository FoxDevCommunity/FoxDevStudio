* The array functions of the language reference.
LOCAL aOne(3), aTwo(2, 2), aDest(3)
aOne(1) = "c"
aOne(2) = "a"
aOne(3) = "b"
? ALEN(aOne), ALEN(aTwo), ALEN(aTwo, 1), ALEN(aTwo, 2)
? ASCAN(aOne, "a"), ASCAN(aOne, "zzz")
ASORT(aOne)
? aOne(1), aOne(2), aOne(3)
? ACOPY(aOne, aDest), aDest(1), aDest(3)
aTwo(1, 1) = "r1c1"
aTwo(2, 2) = "r2c2"
? AELEMENT(aTwo, 2, 2), ASUBSCRIPT(aTwo, 4, 1), ASUBSCRIPT(aTwo, 4, 2)
? ADEL(aOne, 1), aOne(1), aOne(2)
? AINS(aOne, 1), aOne(2)
DIMENSION aOne(5)
? ALEN(aOne)
LOCAL aFonts(1)
? AFONT(aFonts) > 0
* COVERS: ACOPY, ADEL, AELEMENT, AFONT, AINS, ALEN, ASCAN, ASORT, ASUBSCRIPT, DIMENSION, LOCAL
