* The kinds of field a program puts on the character screen for a person to change. None of
* them is read until a READ activates it, and a READ stops to ask a person, so what a golden
* can watch is that each is accepted and that the variable behind it is left as it was.
PUBLIC lLoud, cPick, nSpin, cEdit, nOpt, cGo, cList
lLoud = .F.
cPick = "Two  "
cList = "Beta "
nSpin = 3
cEdit = "some words"
nOpt = 1
cGo = "Save  "
DIMENSION aItems[3]
aItems[1] = "Alpha"
aItems[2] = "Beta"
aItems[3] = "Gamma"
CLEAR

* a check box, a popup of choices, a row of command buttons and a list
@ 1, 2 GET lLoud FUNCTION "*C Loud"
@ 2, 2 GET cPick FUNCTION "^ One;Two;Three"
@ 3, 2 GET cGo FUNCTION "*H Save;Cancel"
@ 4, 2 GET cList FROM aItems SIZE 3, 10
@ 5, 2 GET nOpt FUNCTION "*RN One;Two"
@ 6, 2 GET nSpin SPINNER 1, 10
@ 7, 2 GET cGo FUNCTION "*T Save;Cancel"
@ 8, 2 EDIT cEdit SIZE 3, 20
? lLoud, ALLTRIM(cPick), ALLTRIM(cList)
? TRANSFORM(nSpin), TRANSFORM(nOpt), ALLTRIM(cGo), ALLTRIM(cEdit)

* a field made from a class of the program's own
@ 10, 2 CLASS mycheck NAME oThing
? lLoud

* a menu of the items an array holds, and a picture from a file
@ 12, 2 MENU aItems, 3 TITLE "Pick"
? TRANSFORM(LEN(TinyBmp()))
? TRANSFORM(STRTOFILE(TinyBmp(), "dot.bmp"))
@ 14, 2 SAY "dot.bmp" BITMAP
? FILE("dot.bmp")

* what is on screen can be redrawn without being read
SHOW GET lLoud
SHOW OBJECT 1
CLEAR GETS
? TRANSFORM(RDLEVEL())
ERASE dot.bmp

* the smallest bitmap there is, so the file the command is given is a real one
FUNCTION TinyBmp
LOCAL cHead
cHead = "BM" + CHR(58) + REPLICATE(CHR(0), 3) + REPLICATE(CHR(0), 4) + CHR(54) + REPLICATE(CHR(0), 3)
cHead = cHead + CHR(40) + REPLICATE(CHR(0), 3) + CHR(1) + REPLICATE(CHR(0), 3) + CHR(1) + REPLICATE(CHR(0), 3)
cHead = cHead + CHR(1) + CHR(0) + CHR(24) + CHR(0) + REPLICATE(CHR(0), 4) + CHR(4) + REPLICATE(CHR(0), 3)
cHead = cHead + REPLICATE(CHR(0), 16)
RETURN cHead + CHR(9) + CHR(0) + CHR(0) + CHR(0)
ENDFUNC

DEFINE CLASS mycheck AS CheckBox
  Caption = "Loud"
ENDDEFINE
* COVERS: @ ... CLASS, @ ... EDIT - Edit Boxes, @ ... GET - Check Boxes,
* COVERS: @ ... GET - Combo Boxes, @ ... GET - Command Buttons, @ ... GET - List Boxes,
* COVERS: @ ... GET - Option Buttons, @ ... GET - Spinners, @ ... GET - Transparent Buttons,
* COVERS: @ ... MENU, @ ... SAY - Pictures & OLE Objects, SHOW GET, SHOW OBJECT
