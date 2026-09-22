* The debugger's commands, seen from the program's side: what a run reports about itself and
* where it hands over. The host that answers a Break here is the golden runner, which continues.
? SET("DEBUG"), SET("STEP"), SET("ECHO"), SET("ASSERTS"), SET("DEBUGOUT") == ""
SET ECHO ON
SET DEBUG OFF
* SET DEBUG is kept for programs written against Visual FoxPro 3 and always reads back ON
? SET("ECHO"), SET("DEBUG")
SET ECHO OFF

* DEBUGOUT says something while the program is being worked on; ASSERT says it only when it
* fails, and only while SET ASSERTS is on
DEBUGOUT "counted", 6 * 7, .T.
SET ASSERTS ON
ASSERT 1 = 1 MESSAGE "never seen"
ASSERT 1 = 2 MESSAGE "the count is wrong"
ASSERT 1 = 2
SET ASSERTS OFF
ASSERT 1 = 2 MESSAGE "not seen with asserts off"
? SET("ASSERTS")

* SET DEBUGOUT TO sends the same lines to a file as well, and closes it when it names none
SET DEBUGOUT TO trace.txt
? SET("DEBUGOUT")
DEBUGOUT "into the file"
SET DEBUGOUT TO
? FILETOSTR("trace.txt") == "into the file" + CHR(13) + CHR(10)

* SUSPEND hands the program to the debugger where it stands; the developer's Continue is what
* brings it back, at the line after the one it stopped at. SET STEP ON does the same.
LOCAL nBefore
nBefore = 41
SUSPEND
nBefore = nBefore + 1
SET STEP ON
? nBefore
DO Deeper WITH nBefore
RESUME

PROCEDURE Deeper
LPARAMETERS nValue
* a breakpoint set on this line would stop here, in a frame of its own
? PROGRAM(), nValue
ENDPROC
* COVERS: ASSERT, DEBUGOUT, FILETOSTR, PROGRAM, RESUME, SET, SET ASSERTS, SET DEBUG,
* COVERS: SET DEBUGOUT, SET ECHO, SET STEP, SUSPEND
