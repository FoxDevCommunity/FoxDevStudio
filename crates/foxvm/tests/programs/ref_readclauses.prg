* What a READ runs around itself: the five procedures its own clauses name.
PUBLIC gnTimes
gnTimes = 0
cName = "Jorge"

@ 2, 2 SAY "Name" GET cName
READ WHEN mayread() SHOW drawn() ACTIVATE started() DEACTIVATE ended() VALID finished()
? "the read is over after", gnTimes, "times"

* a WHEN that says no means the read never happens
gnTimes = 0
@ 4, 2 SAY "Again" GET cName
READ WHEN .F. SHOW drawn()
? "nothing was drawn:", gnTimes = 0
CLEAR GETS

PROCEDURE mayread
? "may I read"
RETURN .T.

PROCEDURE drawn
gnTimes = gnTimes + 1
? "drawn"
RETURN .T.

PROCEDURE started
? "started"
RETURN .T.

PROCEDURE ended
? "ended"
RETURN .T.

* the first time round it says no, so the read happens again
PROCEDURE finished
RETURN gnTimes > 1

* COVERS: READ, ReadActivate, ReadDeactivate, ReadShow, ReadValid, ReadWhen
