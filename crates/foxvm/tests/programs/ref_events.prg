* The commands that hang a command off something that may happen, and the keyboard buffer.
ON ERROR ? "went wrong"
ON ESCAPE ? "stopped"
ON SHUTDOWN QUIT
ON READERROR ? "bad value"
ON PAGE AT LINE 55 DO header
ON KEY LABEL F5 DO refresh
ON KEY LABEL CTRL+W ? "closed"
ON KEY = 27 ? "escape"

? ON("ERROR")
? ON("ESCAPE")
? ON("SHUTDOWN")
? ON("READERROR")
? ON("PAGE", "55")
? ON("KEY", "F5")
? ON("KEY", "CTRL+W")
? ON("KEY", "27")

* what is kept and put back
PUSH KEY CLEAR
? ON("KEY", "F5") == "", ON("ESCAPE") == ""
POP KEY
? ON("KEY", "F5")

* and one taken away by naming nothing after it
ON ESCAPE
? ON("ESCAPE") == ""

* keys as if they had been typed, with the names in braces read as the keys they stand for
KEYBOARD "abc"
KEYBOARD "{ENTER}"
KEYBOARD "raw{ENTER}" PLAIN
KEYBOARD "" CLEAR
? "keyboard taken"

* COVERS: KEYBOARD, ON, ON ESCAPE, ON KEY, ON KEY =, ON KEY LABEL, ON PAGE, ON READERROR,
* COVERS: ON SHUTDOWN, POP KEY, PUSH KEY
