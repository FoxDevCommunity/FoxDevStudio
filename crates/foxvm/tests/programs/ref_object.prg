* The object, type and environment functions of the language reference.
LOCAL o
o = CREATEOBJECT("Empty")
? VARTYPE(o), TYPE("o")
? ADDPROPERTY(o, "Caption", "hello")
? o.Caption
? PEMSTATUS(o, "Caption", 5)
o.Caption = "changed"
? o.Caption
LOCAL oNew
oNew = NEWOBJECT("Empty")
? VARTYPE(oNew)
? ISNULL(.NULL.), ISNULL(1)
* the name depends on what compiled it; the level does not
? LEN(PROGRAM()) > 0, PROGRAM(-1)
? PCOUNT(), PARAMETERS()
? LEN(VERSION()) > 0, LEN(OS()) > 0
? JUSTFNAME("c:\dir\file.txt"), JUSTSTEM("c:\dir\file.txt"), JUSTEXT("c:\dir\file.txt")
? JUSTPATH("c:\dir\file.txt"), ADDBS("c:\dir"), FORCEEXT("report.txt", "prg")
? FORCEPATH("file.txt", "c:\other")
? SET("EXACT"), SET("TALK")
? ON("ERROR") == ""
? LINENO() > 0
* COVERS: ADDBS, ADDPROPERTY, CREATEOBJECT, FORCEEXT, FORCEPATH, ISNULL, JUSTEXT, JUSTFNAME,
* COVERS: JUSTPATH, JUSTSTEM, LEN, LINENO, LOCAL, NEWOBJECT, ON, OS, PARAMETERS, PCOUNT,
* COVERS: PEMSTATUS, PROGRAM, SET, TYPE, VARTYPE, VERSION
