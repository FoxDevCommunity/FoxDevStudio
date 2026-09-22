* The character screen, the windows a program opens on it, and the fields it reads from them.
CLEAR
@ 1, 2 SAY "Customer"
@ 2, 2 SAY 1234.5 PICTURE "9,999.99"
? ROW(), COL()
? SROWS(), SCOLS()
? PROW(), PCOL()

* a frame, a region cleared, a region filled and a region moved
@ 4, 0, 8, 30 BOX "-|++++"
@ 5, 1 TO 7, 20 DOUBLE
@ 9, 0 SAY "scratch"
@ 9, 0 CLEAR TO 9, 20
@ 10, 0 FILL TO 10, 5
@ 10, 0 SCROLL TO 12, 20 UP 1

* a window of its own, which takes the output while it is up
DEFINE WINDOW wnote FROM 3, 40 TO 12, 78 TITLE "Notes" DOUBLE CLOSE FLOAT
ACTIVATE WINDOW wnote
@ 0, 1 SAY "in the window"
? WOUTPUT(), WONTOP(), WEXIST("wnote"), WVISIBLE("wnote")
? WROWS("wnote"), WCOLS("wnote"), WLROW("wnote"), WLCOL("wnote")
? WTITLE("wnote"), WPARENT("wnote"), WCHILD("wnote")
? WBORDER("wnote"), WMAXIMUM("wnote"), WMINIMUM("wnote"), WREAD("wnote")
? WDOCKABLE("wnote")

MOVE WINDOW wnote TO 4, 41
SIZE WINDOW wnote TO 8, 30
ZOOM WINDOW wnote MAX
? WLROW("wnote"), WLCOL("wnote"), WROWS("wnote"), WMAXIMUM("wnote")
ZOOM WINDOW wnote NORM
MODIFY WINDOW wnote TITLE "Still notes"
? WTITLE("wnote")

SAVE WINDOWS TO wkept
HIDE WINDOW wnote
? WVISIBLE("wnote")
SHOW WINDOW wnote
? WVISIBLE("wnote")
DEACTIVATE WINDOW wnote
? WOUTPUT() == "", WLAST()
ACTIVATE SCREEN

* the screen itself can be kept and put back
SAVE SCREEN
CLEAR
@ 0, 0 SAY "wiped"
RESTORE SCREEN
RESTORE WINDOW wkept
? WEXIST("wnote")

* the fields a READ walks, and what the program can ask about them
cName = PADR("Acme", 12)
nRate = 5
@ 14, 2 SAY "Name" GET cName PICTURE "@!"
@ 15, 2 SAY "Rate" GET nRate PICTURE "999" VALID nRate > 0
? OBJNUM("cName"), OBJVAR(1), OBJVAR(2)
? RDLEVEL()
READ
? ALLTRIM(cName), nRate
? VARREAD(), RDLEVEL()
SHOW GETS
CLEAR GETS
? OBJNUM("cName")

* a choice put up a line at a time
@ 18, 2 PROMPT "Add"
@ 19, 2 PROMPT "Quit"
nPick = 0
MENU TO nPick
? nPick

* the mouse, and the keys a command can hang off
? MROW(), MCOL(), MDOWN(), MWINDOW() == ""
? FKMAX(), FKLABEL(1), FKLABEL(10)
? TXTWIDTH("hello")
RELEASE WINDOWS

* COVERS: @ ... BOX, @ ... CLEAR, @ ... FILL, @ ... GET - Text Boxes, @ ... PROMPT,
* COVERS: @ ... SAY, @ ... SCROLL, @ ... TO, ACTIVATE SCREEN, ACTIVATE WINDOW, CLEAR,
* COVERS: COL, DEACTIVATE WINDOW, DEFINE WINDOW, FKLABEL, FKMAX, HIDE WINDOW, MCOL,
* COVERS: MDOWN, MENU TO, MODIFY WINDOW, MOVE WINDOW, MROW, MWINDOW, OBJNUM, OBJVAR,
* COVERS: PCOL, PROW, RDLEVEL, READ, RELEASE WINDOWS, RESTORE SCREEN, RESTORE WINDOW,
* COVERS: ROW, SAVE SCREEN, SAVE WINDOWS, SCOLS, SHOW GETS, SHOW WINDOW, SIZE WINDOW,
* COVERS: SROWS, TXTWIDTH, VARREAD, WBORDER, WCHILD, WCOLS, WDOCKABLE, WEXIST, WLAST,
* COVERS: WLCOL, WLROW, WMAXIMUM, WMINIMUM, WONTOP, WOUTPUT, WPARENT, WREAD, WROWS,
* COVERS: WTITLE, WVISIBLE, ZOOM WINDOW
