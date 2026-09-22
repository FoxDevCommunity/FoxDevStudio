ON ERROR ?? ""
SET ALTERNATE TO week1.txt
SET ALTERNATE ON
LOCAL i, j, d, t, s
t = {^2026-09-07 13:05:09}
? "--- dow/week under fdow 1..7, fweek 1..3"
FOR i = 1 TO 7
  SET FDOW TO i
  FOR j = 1 TO 3
    SET FWEEK TO j
    s = "fdow " + STR(i,1) + " fweek " + STR(j,1) + " "
    s = s + "[" + STR(DOW({^2026-09-07}),1) + "]"
    s = s + "[" + STR(DOW({^2026-09-06}),1) + "]"
    s = s + "[" + STR(WEEK({^2026-01-01}),2) + "]"
    s = s + "[" + STR(WEEK({^2026-09-07}),2) + "]"
    s = s + "[" + STR(WEEK({^2027-01-01}),2) + "]"
    s = s + "[" + STR(WEEK({^2021-01-01}),2) + "]"
    s = s + "[" + STR(WEEK({^2021-01-04}),2) + "]"
    ? s
  ENDFOR
ENDFOR
SET FDOW TO 3
SET FWEEK TO 2
? "readback [" + TRANSFORM(SET("FDOW")) + "][" + TRANSFORM(SET("FWEEK")) + "]"
? "explicit args win [" + STR(DOW({^2026-09-07}, 1),1) + "][" + STR(WEEK({^2026-01-01}, 1, 1),2) + "]"
? "cdow/cmonth [" + CDOW({^2026-09-07}) + "][" + CMONTH({^2026-09-07}) + "]"
SET FDOW TO
SET FWEEK TO
? "reset [" + TRANSFORM(SET("FDOW")) + "][" + TRANSFORM(SET("FWEEK")) + "][" + STR(DOW({^2026-09-07}),1) + "]"
? "--- mdy/dmy padding"
? "[" + MDY({^2026-09-07}) + "][" + DMY({^2026-09-07}) + "][" + MDY({^2026-12-25}) + "][" + DMY({^2026-12-25}) + "]"
? "--- ttoc 2 under hours/seconds"
SET HOURS TO 24
? "24 [" + TTOC(t,2) + "]"
SET SECONDS OFF
? "24/nosec [" + TTOC(t,2) + "][" + TTOC({/:},2) + "]"
SET HOURS TO 12
? "12/nosec [" + TTOC(t,2) + "][" + TTOC({/:}) + "]"
SET HOURS TO 24
? "24/nosec empty [" + TTOC({/:}) + "]"
SET SECONDS ON
? "24/sec empty [" + TTOC({/:}) + "]"
SET HOURS TO 12
? "--- empty date under formats"
SET DATE TO GERMAN
? "german [" + DTOC({/}) + "][" + TTOC({/:}) + "]"
SET DATE TO ITALIAN
? "italian [" + DTOC({/}) + "]"
SET DATE TO SHORT
? "short [" + DTOC({/}) + "][" + DTOC({^2026-09-07}) + "][" + TTOC({/:}) + "]"
SET DATE TO LONG
? "long [" + DTOC({/}) + "]"
SET DATE TO AMERICAN
SET MARK TO "*"
? "mark [" + DTOC({/}) + "][" + TTOC({/:}) + "]"
SET MARK TO
? "--- ctod under date/mark"
SET DATE TO GERMAN
? "german [" + DTOC(CTOD("07.09.26")) + "][" + DTOC(CTOD("07/09/26")) + "]"
SET MARK TO "-"
? "german mark - [" + DTOC(CTOD("07-09-26")) + "][" + DTOC(CTOD("07.09.26")) + "]"
SET MARK TO
SET DATE TO AMERICAN
? "--- str/time"
SET HOURS TO 24
? "[" + TIME() + "]"
SET HOURS TO 12
? "--- set date bad word"
SET DATE TO AMERICAN
? "before [" + SET("DATE") + "]"
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
