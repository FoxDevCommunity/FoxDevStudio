ON ERROR ?? ""
SET ALTERNATE TO week2.txt
SET ALTERNATE ON
LOCAL i, j, s
? "--- dow(d,0) and week(d,0,0) follow SET FDOW / SET FWEEK"
FOR i = 1 TO 7
  SET FDOW TO i
  s = "fdow " + STR(i,1) + " [" + STR(DOW({^2026-09-07}, 0),1) + "][" + STR(DOW({^2026-09-06}, 0),1) + "]"
  s = s + "[" + STR(WEEK({^2026-01-01}, 0, 0),2) + "][" + STR(WEEK({^2021-01-04}, 0, 0),2) + "]"
  ? s
ENDFOR
SET FDOW TO 1
FOR j = 1 TO 3
  SET FWEEK TO j
  s = "fweek " + STR(j,1) + " [" + STR(WEEK({^2026-01-01}, 0, 0),2) + "][" + STR(WEEK({^2021-01-01}, 0, 0),2) + "]"
  s = s + "[" + STR(WEEK({^2021-01-04}, 0, 0),2) + "][" + STR(WEEK({^2021-01-04}, 0, 1),2) + "][" + STR(WEEK({^2021-01-04}, 1, 0),2) + "]"
  ? s
ENDFOR
SET FDOW TO 2
SET FWEEK TO 2
? "iso [" + STR(WEEK({^2021-01-04}, 0, 0),2) + "][" + STR(WEEK({^2021-01-01}, 0, 0),2) + "][" + STR(WEEK({^2026-12-31}, 0, 0),2) + "]"
? "out of range [" + STR(DOW({^2026-09-07}, 9),1) + "]"
SET FDOW TO
SET FWEEK TO
? "--- transform corners"
? "[" + TRANSFORM(1234.56, "@$") + "][" + TRANSFORM(1234.56, "@$ ") + "][" + TRANSFORM($1234.56, "@$ 999,999.99") + "]"
? "[" + TRANSFORM(.T.) + "][" + TRANSFORM(.F.) + "][" + TRANSFORM("ab") + "]"
? "[" + TRANSFORM(12, "@L 9999") + "][" + TRANSFORM(-12, "@L 9999") + "][" + TRANSFORM(0, "@Z 9999") + "][" + TRANSFORM(12, "@B 9999") + "]"
SET POINT TO ","
SET SEPARATOR TO "."
? "euro [" + TRANSFORM(1234.5, "999,999.99") + "][" + TRANSFORM(1234.5) + "][" + STR(1234.5, 10, 2) + "][" + TRANSFORM($1234.5) + "]"
SET POINT TO
SET SEPARATOR TO
? "--- val and str with point"
SET POINT TO ","
? "[" + STR(VAL("3.7"), 6, 2) + "][" + STR(VAL("3,7"), 6, 2) + "]"
SET POINT TO
? "--- money"
? "[" + TRANSFORM($0) + "][" + TRANSFORM($-1.5) + "][" + TRANSFORM($1000000) + "]"
SET CURRENCY TO "kr"
SET CURRENCY RIGHT
? "[" + TRANSFORM($0) + "][" + TRANSFORM($-1.5) + "][" + TRANSFORM($1000000) + "]"
SET CURRENCY TO
SET CURRENCY LEFT
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
