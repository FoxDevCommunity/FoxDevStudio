* COVERS: LINENO, MEMORY, MESSAGE, OS
* LINENO(1): counted from the first line the current procedure runs, not from the top of the
* main program the way plain LINENO() is.
? "top", LTRIM(STR(LINENO()))
DO Deep

* MEMORY(): a DOS-era constant, always 640. MEMORY(n): a real reading of memory left, which
* moves between machines and even between runs of the same one - only its shape is measured.
? "MEMORY() is the fixed 640", MEMORY() == 640
? "MEMORY(1) is a number greater than zero", VARTYPE(MEMORY(1)) == "N" AND MEMORY(1) > 0

* MESSAGE(1): the offending source line, in principle - but a .prg run this way (not stepped
* through in the IDE) never has one to give back, whatever kind of error it was.
PUBLIC pcPlain, pcLine
ON ERROR DO CatchPlain
? "x" + 5
ON ERROR DO CatchLine
? "x" + 5
ON ERROR
? "MESSAGE()", pcPlain
? "MESSAGE(1) outside the IDE", "[" + pcLine + "]"

* OS(n): 1 is the name and version, 3 through 5 the version and build apart, and 2 and 6
* through 11 measure as fixed values on any Windows from 8 up, because GetVersionEx lies about
* the version to a program that has not asked to be told the truth
? "OS() names Windows", "WINDOWS" $ UPPER(OS())
? "OS(2)", "[" + OS(2) + "]"
? "OS(3) is numeric text", VARTYPE(VAL(OS(3))) == "N"
? "OS(6)", OS(6)
? "OS(7)", "[" + OS(7) + "]"
? "OS(8)", OS(8)
? "OS(9)", OS(9)
? "OS(10)", OS(10)
? "OS(11)", OS(11)
RETURN

PROCEDURE Deep
? "LINENO() from the main program", LTRIM(STR(LINENO()))
? "LINENO(1) from this procedure's own line", LTRIM(STR(LINENO(1)))
ENDPROC

PROCEDURE CatchPlain
pcPlain = MESSAGE()
ENDPROC

PROCEDURE CatchLine
pcLine = MESSAGE(1)
ENDPROC
