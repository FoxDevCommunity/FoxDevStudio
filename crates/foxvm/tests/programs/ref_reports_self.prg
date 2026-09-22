* The arrays that report on the program itself.
DIMENSION aLibs(1), aOnly(1), aVerbs(1), aFuncs(1), aEvents(1), aWhat(1), aProcs(1), aDefs(1), aFixed(1), aOpen(1)
DECLARE INTEGER GetTickCount IN kernel32
DECLARE INTEGER Sleep IN kernel32 INTEGER dwMilliseconds
? ADLLS(aLibs), ALEN(aLibs, 2)
? aLibs(1, 1), aLibs(1, 2)
? ADLLS(aOnly, "user32")

* the language itself, out of the parser and the library it is built from
? ALANGUAGE(aVerbs, 1) > 100
? ASCAN(aVerbs, "SCAN") > 0
? ALANGUAGE(aFuncs, 2) > 300, ALEN(aFuncs, 2)
? ALANGUAGE(aEvents, 4) > 20
? ASCAN(aEvents, "OpenData") > 0

* what is in a program file, written here so there is one to look at
cSrc = "#DEFINE MAX 10" + CHR(13) + CHR(10) + "DEFINE CLASS Thing AS Custom" + CHR(13) + CHR(10) + ;
    "PROCEDURE Init" + CHR(13) + CHR(10) + "ENDDEFINE"
= STRTOFILE(cSrc, "outline.prg")
? APROCINFO(aWhat, "outline.prg", 1), aWhat(1, 1), aWhat(1, 3)
? APROCINFO(aProcs, "outline.prg", 2), aProcs(1)
? APROCINFO(aDefs, "outline.prg", 3), aDefs(1, 1)

* nothing docks, and no connection is open
? ADOCKSTATE(aFixed), ASQLHANDLES(aOpen)

* COVERS: ADLLS, ADOCKSTATE, ALANGUAGE, APROCINFO, ASQLHANDLES
