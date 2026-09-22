* What a line of a program can be besides a command: a comment, an expression worked out for
* its own sake, a name put together while the program runs, and a class defined where it is used.
NOTE this line is a comment as well, and the words in it are not commands
&& so is this one
LOCAL cName, oThing
* a PRIVATE is visible to what this program calls, which is how Bump below reaches it
PRIVATE x
x = 5
* the = command works an expression out and throws the answer away, which is how a program
* calls something for what it does rather than for what it gives back
= Bump()
? TRANSFORM(x)                          && a comment after a command
= Bump()
? TRANSFORM(x)

* & puts the text of a variable into the line before the line is read
cName = "x"
? TRANSFORM(&cName)
cName = "TRANSFORM(x + 1)"
? &cName
cName = "up"
? Say&cName.()

* EXTERNAL says a name is defined somewhere else, which is a note to whatever builds the program
EXTERNAL PROCEDURE Bump
EXTERNAL ARRAY aNowhere

* DECLARE - DLL says what a function in a library looks like, so a program can call it
DECLARE INTEGER GetCurrentProcessId IN WIN32API
DIMENSION aLibs[1]
? ADLLS(aLibs) > 0

* DEFINE CLASS makes a class out of the program itself
oThing = CREATEOBJECT("Counter")
* the methods a DEFINE CLASS declares are run by the host, and the one this runner uses does
* not run them, so what is watched here is the class itself: its name, its parent and what it
* declares
? TRANSFORM(oThing.Total)
? oThing.Class, TRANSFORM(PEMSTATUS(oThing, "Total", 5))

PROCEDURE Bump
x = x + 1
ENDPROC

FUNCTION Sayup
RETURN "up and away"
ENDFUNC

DEFINE CLASS Counter AS Custom
  Total = 0
ENDDEFINE
* COVERS: &, &&, *, =, DECLARE, DECLARE - DLL, DEFINE CLASS, EXTERNAL, NOTE
