ON ERROR ?? ""
SET ALTERNATE TO date1.txt
SET ALTERNATE ON
LOCAL i, j, d, t, cSet
d = {^2026-09-07}
t = {^2026-09-07 13:05:09}
DIMENSION aF[14]
aF[1]  = "AMERICAN"
aF[2]  = "ANSI"
aF[3]  = "BRITISH"
aF[4]  = "FRENCH"
aF[5]  = "GERMAN"
aF[6]  = "ITALIAN"
aF[7]  = "JAPAN"
aF[8]  = "TAIWAN"
aF[9]  = "USA"
aF[10] = "MDY"
aF[11] = "DMY"
aF[12] = "YMD"
aF[13] = "SHORT"
aF[14] = "LONG"
SET CENTURY OFF
? "--- century off"
FOR i = 1 TO 14
  cSet = aF[i]
  SET DATE TO &cSet
  ? PADR(aF[i], 10) + "[" + DTOC(d) + "][" + TTOC(t) + "][" + SET("DATE") + "]"
ENDFOR
SET CENTURY ON
? "--- century on"
FOR i = 1 TO 14
  cSet = aF[i]
  SET DATE TO &cSet
  ? PADR(aF[i], 10) + "[" + DTOC(d) + "][" + TTOC(t) + "]"
ENDFOR
SET DATE TO AMERICAN
SET CENTURY OFF
? "--- mark"
SET MARK TO "-"
FOR i = 1 TO 12
  cSet = aF[i]
  SET DATE TO &cSet
  ? PADR(aF[i], 10) + "[" + DTOC(d) + "][" + TTOC(t) + "]"
ENDFOR
SET DATE TO AMERICAN
SET MARK TO "*"
? "star [" + DTOC(d) + "][" + TTOC(t) + "][" + SET("MARK") + "]"
SET MARK TO
? "reset [" + DTOC(d) + "][" + SET("MARK") + "]"
? "--- hours and seconds"
SET HOURS TO 12
SET SECONDS ON
? "12/on [" + TTOC(t) + "][" + TTOC({^2026-09-07 00:00:00}) + "][" + TTOC({^2026-09-07 12:00:00}) + "][" + TTOC({^2026-09-07 23:59:59}) + "]"
SET SECONDS OFF
? "12/off [" + TTOC(t) + "][" + TTOC({^2026-09-07 00:00:00}) + "][" + TTOC({^2026-09-07 12:00:00}) + "][" + TTOC({^2026-09-07 23:59:59}) + "]"
SET HOURS TO 24
SET SECONDS ON
? "24/on [" + TTOC(t) + "][" + TTOC({^2026-09-07 00:00:00}) + "][" + TTOC({^2026-09-07 12:00:00}) + "][" + TTOC({^2026-09-07 23:59:59}) + "]"
SET SECONDS OFF
? "24/off [" + TTOC(t) + "][" + TTOC({^2026-09-07 00:00:00}) + "][" + TTOC({^2026-09-07 12:00:00}) + "][" + TTOC({^2026-09-07 23:59:59}) + "]"
SET HOURS TO
SET SECONDS ON
? "--- empty date and datetime"
? "[" + DTOC({/}) + "][" + TTOC({/:}) + "][" + TRANSFORM({/}) + "][" + TRANSFORM({/:}) + "]"
SET CENTURY ON
? "cent on [" + DTOC({/}) + "][" + TTOC({/:}) + "]"
SET CENTURY OFF
? "--- transform of dates"
? "[" + TRANSFORM(d) + "][" + TRANSFORM(t) + "][" + TRANSFORM(d, "@D") + "][" + TRANSFORM(t, "@D") + "]"
? "--- dtos, dtoc 1, ttoc 1, ttoc 2"
? "[" + DTOS(d) + "][" + DTOC(d, 1) + "][" + TTOC(t, 1) + "][" + TTOC(t, 2) + "]"
? "--- mdy dmy cdow cmonth"
? "[" + MDY(d) + "][" + DMY(d) + "][" + CDOW(d) + "][" + CMONTH(d) + "]"
SET CENTURY ON
? "cent on [" + MDY(d) + "][" + DMY(d) + "]"
SET CENTURY OFF
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
