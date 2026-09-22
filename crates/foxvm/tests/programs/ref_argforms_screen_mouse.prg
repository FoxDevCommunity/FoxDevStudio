* The mouse and field functions' argument forms a golden had not yet called: MCOL/MROW with a
* window name and a scale mode, MWINDOW asked about a window by name, OBJNUM asked at a read
* level, and TXTWIDTH asked about a font rather than the default one.
CLEAR
DEFINE WINDOW wm FROM 2, 2 TO 10, 40 TITLE "M"
ACTIVATE WINDOW wm NOSHOW

* MCOL/MROW answer about the mouse pointer, whose position this probe cannot pin down, so what
* is measured is that each form is accepted and answers a number - not which number
? VARTYPE(MCOL(0)) == "N"
? VARTYPE(MCOL(0, 0)) == "N"
? VARTYPE(MCOL(0, 3)) == "N"
? VARTYPE(MCOL("wm")) == "N"
? VARTYPE(MCOL("wm", 0)) == "N"
? VARTYPE(MCOL("wm", 3)) == "N"

? VARTYPE(MROW(0)) == "N"
? VARTYPE(MROW(0, 0)) == "N"
? VARTYPE(MROW(0, 3)) == "N"
? VARTYPE(MROW("wm")) == "N"
? VARTYPE(MROW("wm", 0)) == "N"
? VARTYPE(MROW("wm", 3)) == "N"

? MWINDOW("wm")

* OBJNUM(cVar, nReadLevel): the level asked for has to be a READ actually running, and with
* none running - the ordinary case for a probe that cannot safely block on one - any level
* number errors rather than answering 0.
cName = "x"
TRY
  ? OBJNUM("cName", 1)
CATCH TO oErr
  ? "caught", TRANSFORM(oErr.ErrorNo)
ENDTRY

* TXTWIDTH(cText, cFontName): a font's name without its size is half of a pair the reference
* asks for together, and the product refuses it as it refuses any other short argument list.
TRY
  ? TXTWIDTH("Hello", "Courier New")
CATCH TO oErr
  ? "caught2", TRANSFORM(oErr.ErrorNo)
ENDTRY

* the three- and four-argument forms measure a real font, which differs by machine; what holds
* anywhere is that the answer is a positive number that grows with the text and the point size
? VARTYPE(TXTWIDTH("Hello", "Arial", 10)) == "N" AND TXTWIDTH("Hello", "Arial", 10) > 0
? TXTWIDTH("Hello World", "Arial", 10) > TXTWIDTH("Hello", "Arial", 10)
? TXTWIDTH("Hello", "Arial", 20) > TXTWIDTH("Hello", "Arial", 10)
? VARTYPE(TXTWIDTH("Hello", "Arial", 10, "B")) == "N"

RELEASE WINDOWS

* COVERS: MCOL, MROW, MWINDOW, OBJNUM, TXTWIDTH
