* The functions that take a path apart, ask where the program is running, and ask the file
* system about a file.
*
* A golden cannot hold the directory it ran in - it is a different one every time - so what is
* asserted about the machine's own paths is their shape, and everything else is a path made up
* here and resolved without touching the disk.

* --- the parts of a path
? JUSTDRIVE("C:\dir\file.txt"), JUSTDRIVE("\\server\share\file.txt")
? "[" + JUSTDRIVE("file.txt") + "]", "[" + JUSTDRIVE("") + "]"

* --- what a relative name means, said relative to a path rather than to the current directory
? FULLPATH("b.txt", "C:\one\two\three.txt")
? FULLPATH("..\b.txt", "C:\one\two\three.txt")
? FULLPATH("C:\already\there.txt", "C:\one\two\three.txt")
? FULLPATH(".\b.txt", "C:\one\two\three.txt")

* --- a path shortened to fit a width, which is what a title bar does with one
? DisplayPath("C:\one\two\three\four\five\file.txt", 20)
? DisplayPath("C:\one\file.txt", 40)
* narrower, and the drive has to go before the folders do
? DisplayPath("C:\one\two\three\four\five\file.txt", 15)
? DisplayPath("C:\one\two\three\four\five\file.txt", 14)
* and narrower still is refused: no width under ten holds a path worth reading
LOCAL oErr, cSaid
TRY
  cSaid = "at 9: " + DisplayPath("C:\one\two\three\four\five\file.txt", 9)
CATCH TO oErr
  cSaid = "at 9: error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid

* --- where the program is running, and where the product was installed. Both name a directory,
* and a directory always ends in a separator; what is before it belongs to the machine.
? VARTYPE(CURDIR()), VARTYPE(HOME())
? RIGHT(HOME(), 1) == "\"

* --- is there a directory of that name. Which directories exist belongs to the machine, so
* what a golden can say is that the question is answered with a logical.
? VARTYPE(DIRECTORY("C:\"))

* --- FSIZE is a question about a field and never about a file, whatever the name looks like
CREATE TABLE staff (name C(10), pay N(8,2))
? LTRIM(STR(FSIZE("name"))), LTRIM(STR(FSIZE("pay"))), LTRIM(STR(FSIZE("nosuchcolumn")))
USE

* --- a file that is not there. FDATE and FTIME refuse, because there is no date to give
TRY
  cSaid = "FDATE: " + DTOC(FDATE("nosuchfile.txt"))
CATCH TO oErr
  cSaid = "FDATE: error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid
TRY
  cSaid = "FTIME: [" + FTIME("nosuchfile.txt") + "]"
CATCH TO oErr
  cSaid = "FTIME: error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid

* --- one this program writes does have a date and a time
STRTOFILE("twenty-seven characters in", "made.txt")
? VARTYPE(FDATE("made.txt")), VARTYPE(FTIME("made.txt"))
* and a file name is not a column name either, so FSIZE still answers 0
? LTRIM(STR(FSIZE("made.txt")))

* --- and what a handle is holding goes to the disk when it is told to
LOCAL nHandle
nHandle = FCREATE("flushed.txt")
FWRITE(nHandle, "held")
? FFLUSH(nHandle)
FCLOSE(nHandle)
* a handle nothing opened is not an error: the call fails the way FFLUSH fails
? FFLUSH(-1), FFLUSH(99)
ERASE made.txt
ERASE flushed.txt

* COVERS: CURDIR, DIRECTORY, DisplayPath, FDATE, FFLUSH, FSIZE, FTIME, FULLPATH, HOME,
* COVERS: JUSTDRIVE
