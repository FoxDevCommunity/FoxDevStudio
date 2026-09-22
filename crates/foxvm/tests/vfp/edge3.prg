ON ERROR ?? "[ERR " + LTRIM(STR(ERROR())) + " " + MESSAGE() + "]"
SET ALTERNATE TO edge3.txt
SET ALTERNATE ON
LOCAL i, s
SET CENTURY ON
? "today [" + DTOC(DATE()) + "]"
? "default century [" + TRANSFORM(SET("CENTURY",1)) + "][" + TRANSFORM(SET("CENTURY",2)) + "]"
? "rollover [" + DTOC(CTOD("09/07/26")) + "][" + DTOC(CTOD("09/07/75")) + "][" + DTOC(CTOD("09/07/76")) + "][" + DTOC(CTOD("09/07/99")) + "][" + DTOC(CTOD("09/07/00")) + "]"
SET CENTURY TO 19 ROLLOVER 50
? "19/50 [" + TRANSFORM(SET("CENTURY",1)) + "][" + TRANSFORM(SET("CENTURY",2)) + "][" + DTOC(CTOD("09/07/26")) + "][" + DTOC(CTOD("09/07/49")) + "][" + DTOC(CTOD("09/07/50")) + "][" + DTOC(CTOD("09/07/99")) + "]"
SET CENTURY TO 18 ROLLOVER 0
? "18/0 [" + DTOC(CTOD("09/07/26")) + "][" + DTOC(CTOD("09/07/99")) + "]"
SET CENTURY TO 20
? "20 [" + TRANSFORM(SET("CENTURY",1)) + "][" + TRANSFORM(SET("CENTURY",2)) + "][" + DTOC(CTOD("09/07/26")) + "]"
SET CENTURY TO
? "reset [" + TRANSFORM(SET("CENTURY",1)) + "][" + TRANSFORM(SET("CENTURY",2)) + "][" + DTOC(CTOD("09/07/26")) + "]"
? "literal [" + DTOC({^2026-09-07}) + "][" + DTOC(CTOD("09/07/1926")) + "]"
? "--- nulldisplay length"
FOR i = 1 TO 30
  s = REPLICATE("x", i)
  SET NULLDISPLAY TO s
  IF SET("NULLDISPLAY") != s
    ? "nulldisplay stops at " + LTRIM(STR(i - 1))
    EXIT
  ENDIF
ENDFOR
SET NULLDISPLAY TO
? "--- taiwan reading"
SET DATE TO TAIWAN
? "[" + DTOC(CTOD("115/09/07")) + "][" + DTOC(CTOD("15/09/07")) + "][" + DTOC(CTOD("2026/09/07")) + "]"
SET DATE TO SHORT
? "short cent on empty [" + DTOC({/}) + "][" + TTOC({/:}) + "]"
SET DATE TO LONG
? "long cent on empty [" + DTOC({/}) + "][" + TTOC({/:}) + "]"
SET DATE TO AMERICAN
SET CENTURY OFF
? "--- point in a picture literal"
SET POINT TO ","
? "[" + TRANSFORM(1234.5, "@R 99.99") + "][" + TRANSFORM(1234.5, "999.99") + "]"
SET POINT TO
? "--- separator without a comma in the picture"
SET SEPARATOR TO "#"
? "[" + TRANSFORM(1234567.8, "9999999.99") + "][" + TRANSFORM(1234567.8) + "]"
SET SEPARATOR TO
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
