* SET PATH TO: the folders a relative name is looked for in after the default directory.
* Nothing here prints a path - the folder a program runs in is a different one every time it is
* measured - so which copy of a table was opened is told by the field in it.
SET SAFETY OFF
LOCAL lcErr, lcMake
LOCAL ARRAY laFound[1]
? "[" + SET("PATH") + "]"
MD fdvone
MD fdvtwo
lcMake = "fdvone\shed"
CREATE TABLE (lcMake) (aone c(1))
USE
lcMake = "fdvtwo\shed"
CREATE TABLE (lcMake) (btwo c(1))
USE
lcMake = "fdvtwo\barn"
CREATE TABLE (lcMake) (ctwo c(1))
USE
CLOSE DATABASES ALL
? TRANSFORM(STRTOFILE("hay", "fdvtwo\hay.txt"))

* the list is kept as it was written, upper-cased, and a comma or a semicolon separates entries
SET PATH TO fdvone,fdvtwo
? "[" + SET("PATH") + "]"
USE shed
? FIELD(1)
USE
SET PATH TO fdvtwo;fdvone
? "[" + SET("PATH") + "]"
USE shed
? FIELD(1)
USE

* the default directory is tried before any of them
CREATE TABLE barn (dhere c(1))
USE
CLOSE DATABASES ALL
USE barn
? FIELD(1)
USE

* a name that carries a folder of its own is not looked for on the path
lcErr = ""
TRY
	USE nowhere\shed
CATCH TO oErr
	lcErr = TRANSFORM(oErr.ErrorNo)
ENDTRY
? lcErr

* what else follows the path, and what does not
SET PATH TO fdvtwo
? FILE("hay.txt")
? FILETOSTR("hay.txt")
? ADIR(laFound, "hay.txt")

* a line that opens with a quote is an expression and the rest of it is dropped
SET PATH TO "fdvone" , "fdvtwo"
? "[" + SET("PATH") + "]"
* there is no ADDITIVE: the word becomes part of the folder's name
SET PATH TO fdvone ADDITIVE
? "[" + SET("PATH") + "]"
* and naming nothing clears the list
SET PATH TO
? "[" + SET("PATH") + "]"
? FILE("hay.txt")
* COVERS: SET PATH
