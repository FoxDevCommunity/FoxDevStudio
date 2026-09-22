* COVERS: EDITSOURCE, FILE, STRTOFILE
* FILE(cFileName, nFlags): nFlags only changes the answer for a file with the Hidden or System
* attribute, which nothing here creates - the flag makes no difference to a plain file.
STRTOFILE("here", "normal.txt")
? "FILE() finds a plain file", FILE("normal.txt")
? "FILE(name, 1) agrees for a plain file", FILE("normal.txt", 1)
? "FILE() misses one that is not there", FILE("nosuchfile.zzz")
? "FILE(name, 1) agrees there too", FILE("nosuchfile.zzz", 1)

* STRTOFILE(text, path, nFlag): 1 appends, 2 leads with a UTF-16LE byte order mark, 4 with a
* UTF-8 one - measured as exactly those bytes ahead of the text, not a re-encoding of it.
STRTOFILE("hello", "a.txt")
? "plain write", FILETOSTR("a.txt")
? "additive appends", STRTOFILE(" world", "a.txt", 1) == 6 AND FILETOSTR("a.txt") == "hello world"
STRTOFILE("fresh", "a.txt", 0)
? "0 overwrites same as omitted", FILETOSTR("a.txt") == "fresh"
? "flag 2 leads with FF FE", STRTOFILE("bom", "b.txt", 2) == 5 ;
  AND ASC(SUBSTR(FILETOSTR("b.txt"), 1, 1)) == 255 AND ASC(SUBSTR(FILETOSTR("b.txt"), 2, 1)) == 254 ;
  AND SUBSTR(FILETOSTR("b.txt"), 3) == "bom"
? "flag 4 leads with EF BB BF", STRTOFILE("utf8bom", "c.txt", 4) == 10 ;
  AND ASC(SUBSTR(FILETOSTR("c.txt"), 1, 1)) == 239 AND ASC(SUBSTR(FILETOSTR("c.txt"), 2, 1)) == 187 ;
  AND ASC(SUBSTR(FILETOSTR("c.txt"), 3, 1)) == 191 AND SUBSTR(FILETOSTR("c.txt"), 4) == "utf8bom"

* EDITSOURCE(cFile [, nLine]): opens the file in the editor and answers whether it did - but the
* answer is numeric, not logical (`?` prints a plain 0, not .F.), and measured as 0 for both
* forms here: a .prg run this way, with nobody watching, has no editor for the file to open in.
? "EDITSOURCE(cFile) answers 0 headless", EDITSOURCE("normal.txt") == 0
? "EDITSOURCE(cFile, nLine) does too", EDITSOURCE("normal.txt", 1) == 0

* EDITSOURCE(cFile, nLine, cClass [, cMethod]): the class and the method say where inside a .vcx
* or .scx the cursor lands, and the Coverage Profiler sample reaches one method of one class that
* way. Four arguments are the most it takes.
? "EDITSOURCE(cFile, nLine, cClass) answers 0", EDITSOURCE("normal.txt", 1, "myclass") == 0
? "so does the four-argument form", EDITSOURCE("normal.txt", 1, "myclass", "mymethod") == 0
