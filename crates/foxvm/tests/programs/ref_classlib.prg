* SET CLASSLIB TO: the class libraries whose classes a program may name.
* fdvclasses.vcx sits beside this program; fdvclasses.gen.prg is what wrote it.
LOCAL lcErr, lcWhich
? "[" + SET("CLASSLIB") + "]"
SET CLASSLIB TO fdvclasses
? JUSTFNAME(STREXTRACT(SET("CLASSLIB"), '"', '"'))
? STREXTRACT(SET("CLASSLIB"), "ALIAS ")
? OCCURS("ALIAS", SET("CLASSLIB"))
* the same library again adds nothing to the list
SET CLASSLIB TO fdvclasses ADDITIVE
? OCCURS("ALIAS", SET("CLASSLIB"))
* ALIAS names the library something other than its file
SET CLASSLIB TO fdvclasses ALIAS mylib
? STREXTRACT(SET("CLASSLIB"), "ALIAS ")
? OCCURS("ALIAS", SET("CLASSLIB"))
* with nothing after it, the libraries are forgotten
SET CLASSLIB TO
? "[" + SET("CLASSLIB") + "]"
* a file that is not there, and what the list says afterwards
lcErr = ""
TRY
	SET CLASSLIB TO nosuchlib
CATCH TO oErr
	lcErr = TRANSFORM(oErr.ErrorNo)
ENDTRY
? lcErr
? "[" + SET("CLASSLIB") + "]"
* the name may be an expression in brackets
lcWhich = "fdvclasses.vcx"
SET CLASSLIB TO (lcWhich)
? JUSTFNAME(STREXTRACT(SET("CLASSLIB"), '"', '"'))
SET CLASSLIB TO
? "[" + SET("CLASSLIB") + "]"
* COVERS: SET CLASSLIB
