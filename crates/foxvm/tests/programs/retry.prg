* RETRY runs the failing statement again, after the handler has fixed what broke it.
PUBLIC gnTries
gnTries = 0
ON ERROR DO Fixer
? "start"
? cLate
? "done"
ON ERROR

* From an ordinary procedure it is the calling statement that runs again.
PUBLIC gnRuns
gnRuns = 0
DO Twice
? "runs: " + LTRIM(STR(gnRuns))

PROCEDURE Fixer
  gnTries = gnTries + 1
  ? "try " + LTRIM(STR(gnTries))
  PUBLIC cLate
  cLate = "arrived"
  RETRY

PROCEDURE Twice
  gnRuns = gnRuns + 1
  IF gnRuns < 3
    RETRY
  ENDIF
* COVERS: DO, IF ... ENDIF, LTRIM, ON ERROR, PROCEDURE, PUBLIC, RETRY, STR
