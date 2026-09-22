ON ERROR ?? ""
SET ALTERNATE TO num2.txt
SET ALTERNATE ON
LOCAL n
* SET() readback, one per line so a failure only loses its own line
? "CURRENCY=[" + SET("CURRENCY") + "]"
? "CURRENCY1=[" + SET("CURRENCY", 1) + "]"
? "POINT=[" + SET("POINT") + "]"
? "SEPARATOR=[" + SET("SEPARATOR") + "]"
? "DECIMALS=[" + SET("DECIMALS") + "]"
? "FIXED=[" + SET("FIXED") + "]"
? "HOURS=[" + SET("HOURS") + "]"
? "SECONDS=[" + SET("SECONDS") + "]"
? "FDOW=[" + SET("FDOW") + "]"
? "FWEEK=[" + SET("FWEEK") + "]"
? "MARK=[" + SET("MARK") + "]"
? "NULLDISPLAY=[" + SET("NULLDISPLAY") + "]"
? "CENTURY=[" + SET("CENTURY") + "]"
? "CENTURY1=[" + SET("CENTURY", 1) + "]"
? "CENTURY2=[" + SET("CENTURY", 2) + "]"
? "DATE=[" + SET("DATE") + "]"
* TRANSFORM with no picture
? "--- transform plain, decimals 0..5"
FOR n = 0 TO 5
  SET DECIMALS TO n
  ? "d" + STR(n,1) + " [" + TRANSFORM(1234.5678) + "][" + TRANSFORM(1/3) + "][" + TRANSFORM(2) + "][" + TRANSFORM($1234.5678) + "][" + TRANSFORM(-1234.5678) + "]"
ENDFOR
SET FIXED ON
? "--- transform plain, fixed on"
FOR n = 0 TO 5
  SET DECIMALS TO n
  ? "d" + STR(n,1) + " [" + TRANSFORM(1234.5678) + "][" + TRANSFORM(1/3) + "][" + TRANSFORM(2) + "][" + TRANSFORM($1234.5678) + "][" + TRANSFORM(-1234.5678) + "]"
ENDFOR
SET FIXED OFF
SET DECIMALS TO 2
? "--- str"
? "[" + STR(1234.5678, 12, 4) + "][" + STR(1234.5678) + "][" + STR(1/3, 8, 4) + "]"
SET POINT TO "_"
? "point [" + STR(1234.5678, 12, 4) + "][" + TRANSFORM(1234.5678) + "][" + TRANSFORM(1/3) + "][" + TRANSFORM(1234.5678, "999,999.99") + "]"
SET SEPARATOR TO "#"
? "sep [" + TRANSFORM(1234.5678, "999,999.99") + "][" + TRANSFORM(1234567.89, "@$ 9,999,999.99") + "][" + TRANSFORM(1234.5678) + "]"
SET POINT TO
SET SEPARATOR TO
? "--- currency pictures, default symbol"
? "[" + TRANSFORM($1234.56) + "][" + TRANSFORM(1234.56, "@$ 999,999.99") + "][" + TRANSFORM(1234.56, "$$$,$$$.99") + "][" + TRANSFORM(1234.56, "@$") + "][" + TRANSFORM($1234.56, "999,999.99") + "]"
SET CURRENCY TO "EUR"
? "eur left [" + TRANSFORM($1234.56) + "][" + TRANSFORM(1234.56, "@$ 999,999.99") + "][" + TRANSFORM(1234.56, "$$$,$$$.99") + "][" + TRANSFORM(-1234.56, "@$ 999,999.99") + "]"
SET CURRENCY RIGHT
? "eur right [" + TRANSFORM($1234.56) + "][" + TRANSFORM(1234.56, "@$ 999,999.99") + "][" + TRANSFORM(1234.56, "$$$,$$$.99") + "][" + TRANSFORM(-1234.56, "@$ 999,999.99") + "]"
SET CURRENCY TO
SET CURRENCY LEFT
? "--- nulldisplay in transform and str"
SET NULLDISPLAY TO "<none>"
? "[" + TRANSFORM(.NULL.) + "][" + TRANSFORM(.NULL., "999.99") + "][" + TRANSFORM(.NULL., "@!") + "]"
SET NULLDISPLAY TO
? "--- big and small"
? "[" + TRANSFORM(0.000001) + "][" + TRANSFORM(1e20) + "][" + TRANSFORM(-0.001) + "][" + TRANSFORM(0.5) + "]"
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
