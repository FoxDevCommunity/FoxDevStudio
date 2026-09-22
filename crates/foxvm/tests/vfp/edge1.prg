ON ERROR ?? "ERR" + STR(ERROR(),5)
SET ALTERNATE TO edge1.txt
SET ALTERNATE ON
SET CENTURY ON
SET DATE TO ANSI
? "ansi cent on empty [" + DTOC({/}) + "][" + TTOC({/:}) + "]"
SET DATE TO TAIWAN
? "taiwan cent on empty [" + DTOC({/}) + "][" + DTOC({^1900-05-04}) + "][" + DTOC({^1911-05-04}) + "]"
SET DATE TO ITALIAN
? "italian cent on [" + DTOC({^2026-09-07}) + "]"
SET DATE TO SHORT
? "short [" + DTOC({^2026-12-25}) + "][" + TTOC({^2026-12-25 09:03:00}) + "][" + DTOC(CTOD("12/25/2026")) + "]"
SET DATE TO LONG
? "long [" + DTOC({^2026-12-25}) + "][" + TTOC({^2026-12-25 09:03:00}) + "]"
SET DATE TO AMERICAN
SET CENTURY OFF
? "--- bad arguments"
? "date bad ["
SET DATE TO NOSUCH
?? SET("DATE") + "]"
? "decimals 19 ["
SET DECIMALS TO 19
?? TRANSFORM(SET("DECIMALS")) + "]"
? "hours 13 ["
SET HOURS TO 13
?? TRANSFORM(SET("HOURS")) + "]"
? "fdow 8 ["
SET FDOW TO 8
?? TRANSFORM(SET("FDOW")) + "]"
? "fweek 4 ["
SET FWEEK TO 4
?? TRANSFORM(SET("FWEEK")) + "]"
SET DECIMALS TO 2
? "--- multi character marks and points"
SET MARK TO "ab"
? "mark ab [" + DTOC({^2026-09-07}) + "][" + SET("MARK") + "]"
SET MARK TO
SET POINT TO "ab"
? "point ab [" + TRANSFORM(1234.5) + "][" + SET("POINT") + "]"
SET POINT TO
SET SEPARATOR TO "ab"
? "sep ab [" + TRANSFORM(1234.5, "9,999.99") + "][" + SET("SEPARATOR") + "]"
SET SEPARATOR TO
SET CURRENCY TO "0123456789"
? "currency 10 [" + SET("CURRENCY",1) + "]"
SET CURRENCY TO "012345678"
? "currency 9 [" + SET("CURRENCY",1) + "][" + TRANSFORM(1234.5, "@$ 99,999.99") + "]"
SET CURRENCY TO
? "--- empty arguments"
SET POINT TO ""
? "point empty [" + TRANSFORM(1234.5) + "][" + SET("POINT") + "]"
SET POINT TO
SET NULLDISPLAY TO "a very long null display string indeed"
? "null [" + TRANSFORM(.NULL.) + "]"
SET NULLDISPLAY TO
? "--- question mark of dates"
? "[" + DTOC({^2026-09-07}) + "]"
? {^2026-09-07}
? {^2026-09-07 13:05:09}
SET HOURS TO 24
? {^2026-09-07 13:05:09}
SET HOURS TO 12
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
