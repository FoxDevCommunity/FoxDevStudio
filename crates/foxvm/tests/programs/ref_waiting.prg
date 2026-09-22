* WAIT, which stops until a key is pressed or a time is up, and the commands that let go of
* what a program set up. The keyboard buffer is what stands in for a person here.
SET SAFETY OFF
SET BELL OFF

* a message that does not stop the program, and one that stops until a key arrives
WAIT WINDOW "working" NOWAIT
KEYBOARD "z"
WAIT "" TO cKey
? "[" + cKey + "]"
WAIT WINDOW "timed" TIMEOUT 0.2
? "[" + TRANSFORM(LASTKEY()) + "]"
WAIT CLEAR
WAIT WINDOW "gone" NOWAIT NOCLEAR
WAIT CLEAR
? "[waited]"

* a procedure file named and then let go of again. RELEASE CLASSLIB and RELEASE LIBRARY are
* not here: each names a file of a kind this runner has none of, and naming one that is not
* open is an error rather than something to watch.
* SET("PROCEDURE") answers with the file's full path, which names the folder the program ran
* in, so what is watched is that naming the file and letting it go both go through
SET PROCEDURE TO other
RELEASE PROCEDURE other
? "[let go]"

* a run of printing that goes nowhere, because the device is the screen. EJECT PAGE is not
* here: it sends a form feed, which on the screen is a page of blank lines and no more.
SET DEVICE TO SCREEN
SET PRINTER OFF
PRINTJOB
  @ 1, 1 SAY "on the page"
ENDPRINTJOB
? "[printed]"

* COVERS: PRINTJOB ... ENDPRINTJOB, RELEASE PROCEDURE, WAIT
