* A General field holds an OLE object, and APPEND GENERAL is how a file gets into one.
* Nothing here renders the object, so what a program can watch is the field filling and
* emptying: naming a file puts its contents in, naming none takes them out again.
SET SAFETY OFF
CREATE TABLE pics (name C(8), pic G)
APPEND BLANK
REPLACE name WITH "one"
? EMPTY(pic), ISBLANK(pic)
? TRANSFORM(STRTOFILE("hello there", "note.txt"))
APPEND GENERAL pic FROM note.txt
? EMPTY(pic), ISBLANK(pic)
APPEND GENERAL pic
? EMPTY(pic), ISBLANK(pic)
* CLASS, DATA and LINK describe the object rather than what goes in it
APPEND GENERAL pic FROM note.txt CLASS "Package" LINK
? EMPTY(pic)
APPEND GENERAL pic FROM note.txt DATA "open"
? EMPTY(pic)
* a second record has a General field of its own, and filling one leaves the other alone
APPEND BLANK
REPLACE name WITH "two"
? EMPTY(pic)
GO TOP
? ALLTRIM(name), EMPTY(pic)
USE
ERASE note.txt
* COVERS: APPEND GENERAL
