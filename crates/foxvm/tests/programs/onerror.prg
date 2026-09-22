ON ERROR DO Handler
? "before"
? undefinedVar
? "after"
x = 1 / 0
? "end"
ON ERROR
TRY
  ? nothere
CATCH TO oErr
  ? "try wins: " + oErr.Message
ENDTRY

PROCEDURE Handler
  ? "handled"
* COVERS: ON ERROR, PROCEDURE, TRY...CATCH...FINALLY
