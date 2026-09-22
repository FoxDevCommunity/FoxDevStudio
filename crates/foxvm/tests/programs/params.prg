DO WithParams WITH 1, 2
DO WithLParams WITH 3
? Sum(1, 2, 3)
? Missing()
DO Show WITH "a", "b"
x = 10
y = 20
DO Swap WITH x, y
? x, y
PRIVATE p
p = 1
DO Bump WITH p
? p
LOCAL l
l = 100
DO Bump WITH l
? l
z = 5
= Inc(@z)
? z
? Fact(5)
? Fact(10)

PROCEDURE WithParams
  PARAMETERS a, b
  ? a + b
  DO ShowA

PROCEDURE ShowA
  ? a

PROCEDURE WithLParams
  LPARAMETERS a
  ? a
  TRY
    DO ShowA
  CATCH TO msg
    ? "hidden: " + msg.Message
  ENDTRY

FUNCTION Sum(a, b, c)
  RETURN a + b + c

FUNCTION Missing
  LPARAMETERS notGiven
  RETURN notGiven

PROCEDURE Show(a, b)
  ? a + b

PROCEDURE Swap(a, b)
  LOCAL t
  t = a
  a = b
  b = t

PROCEDURE Bump(v)
  v = v + 1

FUNCTION Inc(v)
  v = v + 1
  RETURN v

FUNCTION Fact(n)
  IF n <= 1
    RETURN 1
  ENDIF
  RETURN n * Fact(n - 1)
* COVERS: DO, FUNCTION, IF ... ENDIF, LOCAL, LPARAMETERS, PARAMETERS, PRIVATE, PROCEDURE,
* COVERS: RETURN, TRY...CATCH...FINALLY
