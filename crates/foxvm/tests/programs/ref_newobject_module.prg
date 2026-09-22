* NEWOBJECT() naming the file its class is to be read out of.
LOCAL o, lcErr
? "[" + SET("CLASSLIB") + "]"
o = NEWOBJECT("fdvgreeter", "fdvclasses.vcx")
? VARTYPE(o)
* the file was read for that one call: it is not one of the loaded libraries afterwards
? "[" + SET("CLASSLIB") + "]"
* a name with no extension is a .vcx
o = NEWOBJECT("fdvgreeter", "fdvclasses")
? VARTYPE(o)
lcErr = ""
TRY
	o = NEWOBJECT("fdvgreeter", "nosuchlib.vcx")
CATCH TO oErr
	lcErr = TRANSFORM(oErr.ErrorNo)
ENDTRY
? lcErr
* COVERS: NEWOBJECT
