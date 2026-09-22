ON ERROR ?? ""
LOCAL i, n
DIMENSION aE[24]
aE[1]  = "1234.5678"
aE[2]  = "-1234.5678"
aE[3]  = "1/3"
aE[4]  = "2"
aE[5]  = "2.00"
aE[6]  = "0"
aE[7]  = "0.5"
aE[8]  = "1000000"
aE[9]  = "10/4"
aE[10] = "1234.5678*1"
aE[11] = "SQRT(2)"
aE[12] = "$1234.5678"
aE[13] = "$1234.56"
aE[14] = "-$1234.56"
aE[15] = "123456789012345678"
aE[16] = "0.000001"
aE[17] = "100000000000000000000"
aE[18] = "1.5+2.25"
aE[19] = "VAL('3.7')"
aE[20] = "PI()"
aE[21] = "-0.001"
aE[22] = "1e20"
aE[23] = "$0"
aE[24] = "10000000.5"
SET ALTERNATE TO num1.txt
SET ALTERNATE ON
FOR i = 1 TO 24
  ? PADR(aE[i], 24) + "["
  ?? EVALUATE(aE[i])
  ?? "]"
ENDFOR
? "--- decimals 0..6, fixed off"
FOR n = 0 TO 6
  SET DECIMALS TO n
  ? STR(n,1) + " ["
  ?? 1/3
  ?? "]["
  ?? 1234.5678
  ?? "]["
  ?? 2
  ?? "]["
  ?? $1234.5678
  ?? "]"
ENDFOR
? "--- decimals 0..6, fixed on"
SET FIXED ON
FOR n = 0 TO 6
  SET DECIMALS TO n
  ? STR(n,1) + " ["
  ?? 1/3
  ?? "]["
  ?? 1234.5678
  ?? "]["
  ?? 2
  ?? "]["
  ?? $1234.5678
  ?? "]"
ENDFOR
SET FIXED OFF
SET DECIMALS TO 2
? "--- point/separator"
SET POINT TO "_"
? "point _ ["
?? 1234.5678
?? "]["
?? 1/3
?? "]["
?? $1234.56
?? "][" + TRANSFORM(1234.5678, "999,999.99") + "]["
?? STR(1234.5678, 12, 4)
?? "]["
?? TRANSFORM(1234.5678)
?? "]"
SET SEPARATOR TO "#"
? "sep # ["
?? 1234.5678
?? "][" + TRANSFORM(1234.5678, "999,999.99") + "][" + TRANSFORM(1234.5678, "@$ 999,999.99") + "]"
SET POINT TO
SET SEPARATOR TO
? "reset ["
?? 1234.5678
?? "][" + TRANSFORM(1234.5678, "999,999.99") + "]"
? "--- currency"
SET CURRENCY TO "DM"
? "DM left ["
?? $1234.56
?? "][" + TRANSFORM(1234.5678, "@$ 999,999.99") + "][" + TRANSFORM($1234.56) + "]"
SET CURRENCY RIGHT
? "DM right ["
?? $1234.56
?? "][" + TRANSFORM(1234.5678, "@$ 999,999.99") + "][" + TRANSFORM($1234.56) + "]"
SET CURRENCY TO
SET CURRENCY LEFT
? "--- nulldisplay"
? "default ["
?? .NULL.
?? "][" + TRANSFORM(.NULL.) + "]"
SET NULLDISPLAY TO "nada"
? "nada ["
?? .NULL.
?? "][" + TRANSFORM(.NULL.) + "][" + SET("NULLDISPLAY") + "]"
SET NULLDISPLAY TO
? "--- set() readback"
? "[" + SET("CURRENCY") + "][" + SET("POINT") + "][" + SET("SEPARATOR") + "][" + SET("DECIMALS") + "][" + SET("FIXED") + "][" + SET("HOURS") + "][" + SET("SECONDS") + "][" + SET("FDOW") + "][" + SET("FWEEK") + "][" + SET("MARK") + "][" + SET("NULLDISPLAY") + "]"
SET CURRENCY TO "DM"
SET CURRENCY RIGHT
? "[" + SET("CURRENCY") + "][" + SET("CURRENCY",1) + "]"
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
