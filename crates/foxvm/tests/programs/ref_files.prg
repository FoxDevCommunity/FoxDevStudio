* Wave 2 of the language reference: the low-level file functions and the file commands.
LOCAL h, lcLine
h = FCREATE("notes.txt")
? h > 0, FERROR()
? FPUTS(h, "first line")
? FWRITE(h, "second")
? FPUTS(h, " line")
? FCLOSE(h)
? FILE("notes.txt")
h = FOPEN("notes.txt")
? FGETS(h)
? FGETS(h)
? FEOF(h)
? FSEEK(h, 0), FSEEK(h, 0, 2)
? FSEEK(h, 6, 0), FREAD(h, 4)
? FCLOSE(h)
* a missing file: -1 and the error number
? FOPEN("nowhere.txt"), FERROR()
* a handle that is not open
? FCLOSE(99), FERROR()
COPY FILE notes.txt TO copy.txt
? FILE("copy.txt")
RENAME copy.txt TO moved.txt
? FILE("copy.txt"), FILE("moved.txt")
h = FOPEN("moved.txt", 2)
? FCHSIZE(h, 5)
? FCLOSE(h)
h = FOPEN("moved.txt")
? FREAD(h, 100)
? FCLOSE(h)
ERASE moved.txt
? FILE("moved.txt")
* the directory
LOCAL aFiles(1)
? ADIR(aFiles, "*.txt")
? aFiles(1, 1), aFiles(1, 2)
? ADIR(aFiles, "*.nothing")
? DEFAULTEXT("report", "prg"), DEFAULTEXT("report.txt", "prg")
? DISKSPACE() > 0, DRIVETYPE("C:")
? LOCFILE("notes.txt")
MD tmpdir
RD tmpdir
* COVERS: ADIR, COPY FILE, DEFAULTEXT, DISKSPACE, DRIVETYPE, ERASE, FCHSIZE, FCLOSE, FCREATE,
* COVERS: FEOF, FERROR, FGETS, FILE, FOPEN, FPUTS, FREAD, FSEEK, FWRITE, LOCAL, LOCFILE, MD, RD,
* COVERS: RENAME
