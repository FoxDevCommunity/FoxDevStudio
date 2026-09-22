x = 1
PUBLIC g
g = "global"
DO Inner
? g
RELEASE g
TRY
  ? g
CATCH TO oErr
  ? oErr.Message
ENDTRY
LOCAL l
l = 3
RELEASE l
? l

PROCEDURE Inner
  ? g
  g = "changed in proc"
* COVERS: DO, LOCAL, PROCEDURE, PUBLIC, RELEASE, TRY...CATCH...FINALLY
