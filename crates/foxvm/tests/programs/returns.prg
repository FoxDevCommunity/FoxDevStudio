? Twice(4)
? NoValue()
DO Early
? "after early"
x = Chain()
? x

FUNCTION Twice(n)
  RETURN n * 2

PROCEDURE NoValue
  RETURN

PROCEDURE Early
  ? "early start"
  RETURN
  ? "not printed"

FUNCTION Chain
  RETURN Twice(Twice(3))
* COVERS: DO, FUNCTION, PROCEDURE, RETURN
