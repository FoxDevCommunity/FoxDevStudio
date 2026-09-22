PRIVATE pShared
pShared = "from main"
LOCAL lHidden
lHidden = "local in main"
DO ShowShared
DO TryHidden
implicit = 5
DO ReadImplicit
DO MakePrivate
? implicit
? pShared
DO ShadowIt
? pShared

PROCEDURE ShowShared
  ? pShared

PROCEDURE TryHidden
  TRY
    ? lHidden
  CATCH TO oErr
    ? "caught: " + oErr.Message
  ENDTRY

PROCEDURE ReadImplicit
  ? implicit * 2
  implicit = implicit + 1
  ? implicit

PROCEDURE MakePrivate
  PRIVATE inner
  inner = "gone"
  ? inner

PROCEDURE ShadowIt
  PRIVATE pShared
  pShared = "shadow"
  DO ShowShared
* COVERS: DO, LOCAL, PRIVATE, PROCEDURE, TRY...CATCH...FINALLY
