* LOADPICTURE() and SAVEPICTURE(): a picture off disk, and one written back.
* fdvfox.bmp is on the stage: four pixels across, three down, twenty-four bits deep.
LOCAL p, q

p = LOADPICTURE("fdvfox.bmp")
? VARTYPE(p)
* Width and Height are HIMETRIC - hundredths of a millimetre - at ninety-six pixels to the inch
? p.Width
? p.Height
* 1 is a bitmap
? p.Type
? VARTYPE(p.Handle)

* with no file it is the null picture, which has no size and no kind
q = LOADPICTURE()
? VARTYPE(q)
? q.Width, q.Height, q.Type

* writing it back gives a bitmap of the same size; the null picture is not one to write
? SAVEPICTURE(p, "copy.bmp")
? SAVEPICTURE(q, "empty.bmp")
? FILE("copy.bmp")
LOCAL r
r = LOADPICTURE("copy.bmp")
? r.Width, r.Height, r.Type

* a file that is not there is error 1, as it is everywhere else
TRY
   ? VARTYPE(LOADPICTURE("nosuch.bmp"))
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY

* COVERS: LOADPICTURE, SAVEPICTURE
