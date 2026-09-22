TRY
  ? "in try"
  THROW "boom"
  ? "skipped"
CATCH TO oErr
  ? "caught " + oErr.Message
FINALLY
  ? "finally"
ENDTRY
TRY
  x = 1 / 0
CATCH TO e
  ? e.Message
ENDTRY
TRY
  TRY
    THROW "inner"
  FINALLY
    ? "inner finally"
  ENDTRY
CATCH TO e
  ? "outer " + e.Message
ENDTRY
? Nested()
FOR i = 1 TO 3
  TRY
    IF i = 2
      EXIT
    ENDIF
    ? i
  FINALLY
    ? "fin " + IIF(i = 1, "one", "two")
  ENDTRY
ENDFOR
TRY
  THROW "bad"
CATCH TO e WHEN e.UserValue = "bad"
  ? "right"
ENDTRY
TRY
  TRY
    THROW "pass"
  CATCH TO e WHEN e.UserValue = "other"
    ? "wrong"
  ENDTRY
CATCH TO e
  ? "outer got " + e.Message
ENDTRY
TRY
  x = 1 / 0
CATCH TO e WHEN e.ErrorNo = 12
  ? "wrong clause"
CATCH TO e WHEN e.ErrorNo = 1307
  ? "divide, error " + TRANSFORM(e.ErrorNo) + " line " + TRANSFORM(e.LineNo)
CATCH
  ? "fallback"
ENDTRY
TRY
  THROW "thrown"
CATCH TO e
  ? TRANSFORM(e.ErrorNo) + " " + e.UserValue
ENDTRY
? "done"

FUNCTION Nested
  TRY
    RETURN "returned"
  FINALLY
    ? "cleanup"
  ENDTRY
  RETURN "not here"
* COVERS: EXIT, FOR ... ENDFOR, FUNCTION, IF ... ENDIF, IIF, RETURN, TRANSFORM,
* COVERS: TRY...CATCH...FINALLY
