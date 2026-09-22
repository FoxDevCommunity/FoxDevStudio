* The window functions' argument forms a golden had not yet called: asking about the active
* output window by leaving the window name out, and WCHILD's own two forms - the count of an
* explicitly named window's children, and the nth of them.
*
* WCOLS/WROWS/WLCOL/WLROW measure in real screen pixels once a window is on real glass, so the
* number itself moves with the font Windows substitutes on the machine; what stays true anywhere
* is that leaving the window name out answers the same thing as naming the active one.
CLEAR
DEFINE WINDOW wa FROM 2, 2 TO 10, 40 TITLE "Parent"
DEFINE WINDOW wb FROM 3, 3 TO 9, 30 TITLE "Child" IN WINDOW wa
ACTIVATE WINDOW wa

* the no-argument forms all answer about the active output window, which is wa
? WBORDER() == WBORDER("wa")
? WCOLS() == WCOLS("wa")
? WROWS() == WROWS("wa")
? WLCOL() == WLCOL("wa")
? WLROW() == WLROW("wa")
? WMAXIMUM()
? WMINIMUM()
? WPARENT()
? WREAD()
? WTITLE()
? WCHILD()

* the one-argument forms of WLAST, WONTOP and WOUTPUT: whether the window named was the one in
* question, once wb has taken over as the active output window
ACTIVATE WINDOW wb
? WLAST("wa")
? WLAST("wb")
? WONTOP("wa")
? WONTOP("wb")
? WOUTPUT("wa")
? WOUTPUT("wb")

* WCHILD(cWindowName [, nChildWindow]): the count of wa's children, then each one by number -
* wa has one, so asking for a second answers the empty string
? WCHILD("wa")
? WCHILD("wa", 1)
? WCHILD("wa", 2)

DEACTIVATE WINDOW wb
DEACTIVATE WINDOW wa
RELEASE WINDOWS

* COVERS: WBORDER, WCHILD, WCOLS, WLAST, WLCOL, WLROW, WMAXIMUM, WMINIMUM, WONTOP, WOUTPUT,
* COVERS: WPARENT, WREAD, WROWS, WTITLE
