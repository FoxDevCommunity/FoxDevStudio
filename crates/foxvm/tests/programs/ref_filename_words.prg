* Where a command expects a file name, what counts as a name and what is read as an expression.
* Nothing here prints a path - the folder a program runs in is a different one every time it is
* measured - so a table is told apart by the field in it.
SET SAFETY OFF
LOCAL lcName, lcErr
CREATE TABLE fdvone (fone c(1))
USE
CREATE TABLE fdvtwo (ftwo c(1))
USE
CLOSE DATABASES ALL
lcName = "fdvtwo"

* a bare word is the name, even when a variable of that name is standing right there
lcErr = ""
TRY
	USE lcName
CATCH TO oErr
	lcErr = TRANSFORM(oErr.ErrorNo)
ENDTRY
? lcErr

* a word with a bracket after it is a function call, and the call's value is the name
USE UPPER("fdvtwo")
? FIELD(1)
USE
USE ALLTRIM(lcName)
? FIELD(1)
USE

* so is a line with a quoted string in it, which is what makes the whole thing an expression
USE lcName + ""
? FIELD(1)
USE
* and a word that is not a variable is then looked for as one
lcErr = ""
TRY
	USE fdvone + ""
CATCH TO oErr
	lcErr = TRANSFORM(oErr.ErrorNo)
ENDTRY
? lcErr

* a name with folders and a suffix in it is still a name
? FILE("fdvone.dbf")
USE fdvone.dbf
? FIELD(1)
USE

* a database container is named the same way
CREATE DATABASE fdvdb
CLOSE DATABASES ALL
OPEN DATABASE UPPER("fdvdb")
? JUSTSTEM(DBC())
CLOSE DATABASES ALL
* COVERS: OPEN DATABASE, USE
