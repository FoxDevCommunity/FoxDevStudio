FOR i = 1 TO 3
  DO CASE
    CASE i = 1
      ? "one"
    CASE i = 2
      ? "two"
    OTHERWISE
      ? "many"
  ENDCASE
ENDFOR
n = 0
?
DO WHILE .T.
  n = n + 1
  IF n % 2 = 0
    LOOP
  ENDIF
  IF n > 7
    EXIT
  ENDIF
  ?? n
ENDDO
?
FOR i = 10 TO 1 STEP -3
  ?? i
  ?? " "
ENDFOR
?
FOR i = 1 TO 4 STEP 2
  ?? i
ENDFOR
n = 3
?
FOR i = 1 TO n
  n = 10
  ?? i
ENDFOR
?
FOR i = 1 TO 2
  FOR j = 1 TO 2
    ?? i * 10 + j
    ?? " "
  ENDFOR
ENDFOR
? i, j
LOCAL k
FOR k = 5 TO 1
  ? "never"
NEXT
? k
?
FOR i = 1 TO 3
  IF i = 2
    LOOP
  ENDIF
  ?? i
ENDFOR
* COVERS: DO CASE ... ENDCASE, DO WHILE ... ENDDO, EXIT, FOR ... ENDFOR, IF ... ENDIF, LOCAL,
* COVERS: LOOP
