* Every setting that decides what a value looks like, through the one formatter.
* The same program runs against vfp9.exe; see crates/foxvm/tests/vfp.
LOCAL i, n, d, t, cWord
d = {^2026-09-07}
t = {^2026-09-07 13:05:09}
DIMENSION aWords[14]
aWords[1]  = "AMERICAN"
aWords[2]  = "ANSI"
aWords[3]  = "BRITISH"
aWords[4]  = "FRENCH"
aWords[5]  = "GERMAN"
aWords[6]  = "ITALIAN"
aWords[7]  = "JAPAN"
aWords[8]  = "TAIWAN"
aWords[9]  = "USA"
aWords[10] = "MDY"
aWords[11] = "DMY"
aWords[12] = "YMD"
aWords[13] = "SHORT"
aWords[14] = "LONG"

? "-- SET DECIMALS and SET FIXED"
FOR n = 0 TO 5
  SET DECIMALS TO n
  ? "off " + STR(n,1) + " [" + TRANSFORM(1/3) + "][" + TRANSFORM(2) + "][" + TRANSFORM(10/4) + "]"
ENDFOR
SET FIXED ON
FOR n = 0 TO 5
  SET DECIMALS TO n
  ? "on  " + STR(n,1) + " [" + TRANSFORM(1/3) + "][" + TRANSFORM(2) + "][" + TRANSFORM(10/4) + "]"
ENDFOR
SET FIXED OFF
SET DECIMALS TO 2
? "[" + TRANSFORM(1/3) + "][" + TRANSFORM(-1/3) + "][" + TRANSFORM(0) + "]"
?? "[" + STR(1234.5678, 12, 4) + "][" + STR(1234.5678) + "][" + STR(1/3, 8, 4) + "]"

? "-- SET POINT and SET SEPARATOR"
SET POINT TO "_"
? "point [" + TRANSFORM(1/3) + "][" + STR(1234.5678, 12, 4) + "][" + TRANSFORM(1234.5678, "999,999.99") + "]"
SET SEPARATOR TO "#"
? "sep [" + TRANSFORM(1234.5678, "999,999.99") + "][" + TRANSFORM(1234567.8, "9,999,999.99") + "]"
SET POINT TO ","
SET SEPARATOR TO "."
? "euro [" + TRANSFORM(1234.5, "999,999.99") + "][" + STR(1234.5, 10, 2) + "][" + STR(VAL("3,7"), 6, 2) + "][" + STR(VAL("3.7"), 6, 2) + "]"
SET POINT TO
SET SEPARATOR TO
? "reset [" + TRANSFORM(1234.5678, "999,999.99") + "][" + STR(VAL("3.7"), 6, 2) + "][" + SET("POINT") + SET("SEPARATOR") + "]"

? "-- SET CURRENCY"
? "dollar [" + TRANSFORM(1234.56, "@$ 999,999.99") + "][" + TRANSFORM(1234.56, "$$$,$$$.99") + "]"
SET CURRENCY TO "EUR"
? "eur left [" + TRANSFORM(1234.56, "@$ 999,999.99") + "][" + TRANSFORM(-1234.56, "@$ 999,999.99") + "]"
SET CURRENCY RIGHT
? "eur right [" + TRANSFORM(1234.56, "@$ 999,999.99") + "][" + TRANSFORM(-1234.56, "@$ 999,999.99") + "]"
? "readback [" + SET("CURRENCY") + "][" + SET("CURRENCY", 1) + "]"
SET CURRENCY TO
SET CURRENCY LEFT
? "reset [" + SET("CURRENCY") + "][" + SET("CURRENCY", 1) + "]"

? "-- SET NULLDISPLAY"
? "default [" + TRANSFORM(.NULL.) + "][" + SET("NULLDISPLAY") + "]"
SET NULLDISPLAY TO "<none>"
? "set [" + TRANSFORM(.NULL.) + "][" + TRANSFORM(.NULL., "999.99") + "][" + SET("NULLDISPLAY") + "]"
SET NULLDISPLAY TO
? "reset [" + TRANSFORM(.NULL.) + "]"

? "-- SET DATE and SET CENTURY"
SET CENTURY OFF
FOR i = 1 TO 14
  cWord = "SET DATE TO " + aWords[i]
  &cWord
  ? PADR(aWords[i], 10) + "[" + DTOC(d) + "][" + TTOC(t) + "][" + SET("DATE") + "]"
ENDFOR
SET CENTURY ON
FOR i = 1 TO 14
  cWord = "SET DATE TO " + aWords[i]
  &cWord
  ? PADR(aWords[i], 10) + "[" + DTOC(d) + "]"
ENDFOR
SET CENTURY OFF
SET DATE TO AMERICAN

? "-- SET MARK TO"
SET MARK TO "*"
? "star [" + DTOC(d) + "][" + TTOC(t) + "][" + SET("MARK") + "]"
SET DATE TO GERMAN
? "german [" + DTOC(d) + "][" + DTOC({}) + "]"
SET DATE TO AMERICAN
SET MARK TO
? "reset [" + DTOC(d) + "][" + SET("MARK") + "]"

? "-- SET HOURS and SET SECONDS"
SET HOURS TO 12
? "12 on  [" + TTOC(t) + "][" + TTOC({^2026-09-07 00:00:00}) + "][" + TTOC(t, 2) + "]"
SET SECONDS OFF
? "12 off [" + TTOC(t) + "][" + TTOC({^2026-09-07 12:00:00}) + "][" + TTOC(t, 2) + "]"
SET HOURS TO 24
? "24 off [" + TTOC(t) + "][" + TTOC(t, 2) + "][" + TRANSFORM(SET("HOURS")) + "]"
SET SECONDS ON
? "24 on  [" + TTOC(t) + "][" + TTOC({^2026-09-07 23:59:59}) + "][" + SET("SECONDS") + "]"
? "empty  [" + TTOC({/:}) + "][" + DTOC({}) + "]"
SET HOURS TO 12
? "empty  [" + TTOC({/:}) + "]"

? "-- SET FDOW and SET FWEEK"
FOR i = 1 TO 7
  SET FDOW TO i
  ? "fdow " + STR(i,1) + " [" + STR(DOW(d, 0),1) + "][" + STR(DOW(d),1) + "][" + STR(WEEK({^2021-01-04}, 0, 0),2) + "]"
ENDFOR
SET FDOW TO 1
FOR i = 1 TO 3
  SET FWEEK TO i
  ? "fweek " + STR(i,1) + " [" + STR(WEEK({^2026-01-01}, 0, 0),2) + "][" + STR(WEEK({^2021-01-04}, 0, 0),2) + "][" + STR(WEEK({^2021-01-04}),2) + "]"
ENDFOR
SET FDOW TO 2
SET FWEEK TO 2
? "iso [" + STR(WEEK({^2021-01-04}, 0, 0),2) + "][" + STR(WEEK({^2021-01-01}, 0, 0),2) + "][" + STR(WEEK({^2026-12-31}, 0, 0),2) + "]"
? "readback [" + TRANSFORM(SET("FDOW")) + "][" + TRANSFORM(SET("FWEEK")) + "]"
SET FDOW TO
SET FWEEK TO
? "reset [" + TRANSFORM(SET("FDOW")) + "][" + TRANSFORM(SET("FWEEK")) + "]"

? "-- SET CENTURY TO ROLLOVER"
SET CENTURY ON
SET CENTURY TO 19 ROLLOVER 50
? "19/50 [" + DTOC(CTOD("09/07/26")) + "][" + DTOC(CTOD("09/07/49")) + "][" + DTOC(CTOD("09/07/50")) + "][" + DTOC(CTOD("09/07/99")) + "]"
SET CENTURY TO 18 ROLLOVER 0
? "18/0 [" + DTOC(CTOD("09/07/26")) + "][" + DTOC(CTOD("09/07/99")) + "]"
? "readback [" + TRANSFORM(SET("CENTURY", 1)) + "][" + TRANSFORM(SET("CENTURY", 2)) + "][" + SET("CENTURY") + "]"
SET CENTURY TO
? "reset [" + DTOC(CTOD("09/07/26")) + "][" + DTOC(CTOD("09/07/1926")) + "]"
SET CENTURY OFF

? "-- the words a date is written in"
? "[" + CDOW(d) + "][" + CMONTH(d) + "][" + MDY(d) + "][" + DMY(d) + "][" + DTOS(d) + "]"
SET CENTURY ON
? "[" + MDY(d) + "][" + DMY(d) + "]"
SET CENTURY OFF
? d
?? " "
?? t
* COVERS: ?, ??, SET CENTURY, SET CURRENCY, SET DATE, SET DECIMALS, SET FDOW, SET FIXED
* COVERS: SET FWEEK, SET HOURS, SET MARK TO, SET NULLDISPLAY, SET POINT, SET SECONDS
* COVERS: SET SEPARATOR, CDOW, CMONTH, CTOD, DMY, DOW, DTOC, DTOS, MDY, SET, STR, TRANSFORM
* COVERS: TTOC, VAL, WEEK
