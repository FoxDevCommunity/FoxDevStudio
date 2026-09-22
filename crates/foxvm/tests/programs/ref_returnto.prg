* RETURN TO leaves every routine between here and the one it names, rather than only the one
* the RETURN is written in. RETURN TO MASTER is not here: which program is the master depends
* on what started the run, and that is not the same under this runner as under Visual FoxPro.
? "main starts"
DO Outer
? "main again"

PROCEDURE Outer
? "  outer"
DO Middle
? "  outer after"
ENDPROC

PROCEDURE Middle
? "    middle"
DO Inner
? "    middle after, which is not reached"
ENDPROC

PROCEDURE Inner
? "      inner"
RETURN TO Outer
ENDPROC


* COVERS: RETURN
