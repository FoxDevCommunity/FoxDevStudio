* COVERS: ADIR, DIRECTORY, DISKSPACE, DISPLAYPATH, FCREATE, FDATE, FFLUSH, FGETS, FPUTS, FULLPATH,
* COVERS: FWRITE, LOCFILE
* The file functions' forms no other golden calls: ADIR with a mask, an attribute and a case
* flag; DIRECTORY and DISKSPACE's second argument; DISPLAYPATH with only the required first one,
* which the reference gives no brackets around at all; FCREATE's attribute; FDATE asking for a
* DateTime; FFLUSH's force flag; FGETS and FPUTS/FWRITE's length; FULLPATH with nothing to
* resolve against; LOCFILE's extension and caption.

* --- ADIR(1): the array alone, defaulting the mask to "*.*" over the current directory. A fresh
* subdirectory keeps the count from depending on whatever else this run staged beside the
* program, and CD back at the end so the rest of this golden still writes where it started.
MD dtest1
CD dtest1
STRTOFILE("hello", "a.txt")
STRTOFILE("world!", "b.dat")
MD childdir
n = ADIR(aFiles)
? "ADIR(1) count", n
? aFiles(1,1), aFiles(1,2), aFiles(1,5)
? aFiles(2,1), aFiles(2,2), aFiles(2,5)

* --- ADIR(3): cAttribute "D" brings the folder's own subdirectories in too, "." and ".." besides,
* mixed in with the files rather than listed after them.
m = ADIR(aFiles2, "*.*", "D")
? "ADIR(3) count", m
FOR i = 1 TO m
  ? aFiles2(i,1), aFiles2(i,2), aFiles2(i,5)
ENDFOR

* --- ADIR(4): nFlag 1 answers each name in the case it was actually written with, not upper-cased
q = ADIR(aFiles3, "*.*", "", 1)
? "ADIR(4) count", q
? aFiles3(1,1)
? aFiles3(2,1)
CD ..

* --- DIRECTORY(2): nFlags makes no difference to a plain, visible directory - only to one with
* the Hidden or System attribute, which nothing here creates
? "DIRECTORY(2) flag0", DIRECTORY("dtest1", 0)
? "DIRECTORY(2) flag1", DIRECTORY("dtest1", 1)
? "DIRECTORY(2) missing", DIRECTORY("nosuchdir4321", 1)

* --- DISKSPACE(1,2): free bytes on a named drive, and each of the three nType answers - a number
* greater than 0, since the exact byte count is this machine's own
? "DISKSPACE(1) drive alone", DISKSPACE("C:") > 0
? "DISKSPACE(2) total", DISKSPACE("C:", 1) > 0
? "DISKSPACE(2) free (default)", DISKSPACE("C:", 2) > 0
? "DISKSPACE(2) user free", DISKSPACE("C:", 3) > 0

* --- DISPLAYPATH(1): the reference gives nMaxLength with no brackets at all, and the product
* holds to that - omitting it is error 1229, "Too few arguments", not a default width
LOCAL oErr, cSaid
TRY
  cSaid = "unreached"
  cSaid = DISPLAYPATH("C:\one\two\file.txt")
CATCH TO oErr
  cSaid = "error " + LTRIM(STR(oErr.ErrorNo)) + " " + oErr.Message
ENDTRY
? "DISPLAYPATH(1)", cSaid

* --- FCREATE(2): an attribute other than 0 sets Windows' own read-only bit, which blocks a write
* to the file even after it is closed and reopened
h1 = FCREATE("ro.txt", 1)
? "FCREATE(2) handle valid", h1 >= 0
? "FPUTS still writes it", FPUTS(h1, "abc") > 0
= FCLOSE(h1)

* --- FDATE(2): 1 asks for the DateTime rather than the Date
STRTOFILE("x", "dated.txt")
? "FDATE(1) type", VARTYPE(FDATE("dated.txt"))
? "FDATE(2) type", VARTYPE(FDATE("dated.txt", 1))
? "FDATE(2,0) same as FDATE(1)", FDATE("dated.txt", 0) == FDATE("dated.txt")

* --- FFLUSH(2): lForce makes no visible difference to a file already flushed to this host
hh = FOPEN("dated.txt", 2)
? "FFLUSH(2) forced", FFLUSH(hh, .T.)
? "FFLUSH(2) not forced", FFLUSH(hh, .F.)
= FCLOSE(hh)

* --- FGETS(2): a length shorter than the line only takes that many bytes of it
STRTOFILE("line one" + CHR(13) + CHR(10) + "line two", "lines.txt")
hh2 = FOPEN("lines.txt")
? "FGETS(2) short", FGETS(hh2, 4)
? "FGETS(1) rest", FGETS(hh2)
? "FGETS(1) next line", FGETS(hh2)
= FCLOSE(hh2)

* --- FPUTS(3): a count shorter than the text only writes that many characters of it
hh3 = FCREATE("puts.txt")
? "FPUTS(3) short", FPUTS(hh3, "abcdef", 3)
? "FPUTS(2) whole", FPUTS(hh3, "xyz")
= FCLOSE(hh3)
? "FPUTS(3) result", FILETOSTR("puts.txt")

* --- FWRITE(3): the same, without the carriage return and line feed FPUTS adds
hh4 = FCREATE("written.txt")
? "FWRITE(3) short", FWRITE(hh4, "abcdef", 3)
? "FWRITE(2) whole", FWRITE(hh4, "xyz")
= FCLOSE(hh4)
? "FWRITE(3) result", FILETOSTR("written.txt")

* --- FULLPATH(1): the file's path from the root, without a second path to resolve it against.
* A golden cannot hold the directory it ran in, so what is asserted is the shape.
? "FULLPATH(1) finds it", "DATED.TXT" $ FULLPATH("dated.txt")
? "FULLPATH(1) upper case", FULLPATH("dated.txt") == UPPER(FULLPATH("dated.txt"))

* --- LOCFILE(2,3): an extension to search with, and a caption nobody sees because the file is
* there. LOCFILE puts up a dialog when the file is not found, so this only ever calls it with
* one that is.
? "LOCFILE(2) finds it", "DATED.TXT" $ UPPER(LOCFILE("dated", "TXT"))
? "LOCFILE(3) finds it", "DATED.TXT" $ UPPER(LOCFILE("dated", "TXT", "Pick a file:"))
