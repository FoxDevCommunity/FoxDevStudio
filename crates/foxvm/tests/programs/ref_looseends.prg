* Two things Visual FoxPro's compiler lets through that ours used to refuse.
*
* The Foundation Classes contain both: `_dataedit2.vcx checkerror` writes `.ALIAS` in a method
* with no WITH anywhere in it, and `_reportlistener.vcx sendfx` ends its LPARAMETERS with a
* stray `)`. Neither file would compile here, so neither class worked at all.

* a name list may be closed with a paren it never opened
LOCAL x, y)
x = 1
y = 2
? TRANSFORM(x + y)

* and `.member` compiles wherever it is written; whether there is a WITH open to hang it on is
* a question for the moment the line runs
? Caption(CREATEOBJECT("Custom"))
? Loose()

FUNCTION Caption
LPARAMETERS toWhat)
WITH toWhat
   .AddProperty("Caption", "from inside a WITH")
   RETURN .Caption
ENDWITH

FUNCTION Loose
LOCAL lcAnswer
lcAnswer = "no error"
TRY
   lcAnswer = .Caption
CATCH TO loErr
   lcAnswer = "error " + TRANSFORM(loErr.ErrorNo)
ENDTRY
RETURN lcAnswer

* COVERS: LOCAL, LPARAMETERS, WITH ... ENDWITH
