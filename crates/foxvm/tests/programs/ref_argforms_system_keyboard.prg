* COVERS: CAPSLOCK, CHRSAW, INKEY, INSMODE, NUMLOCK, READKEY
* The keyboard-state functions' optional argument: CAPSLOCK(), NUMLOCK() and INSMODE() take a
* new state and answer the one they had before. That return value is all a golden can trust -
* measuring this on the real product found NUM LOCK's own key visibly follow the call and CAPS
* LOCK's not, on the very same machine - so nothing here reads the key back, only the value
* handed back before it changed, and each is restored to what it started at.
LOCAL lOld, lReturned
lOld = CAPSLOCK()
lReturned = CAPSLOCK(!lOld)
? "CAPSLOCK returns the state from before the change", lReturned == lOld
CAPSLOCK(lOld)

lOld = NUMLOCK()
lReturned = NUMLOCK(!lOld)
? "NUMLOCK returns the state from before the change", lReturned == lOld
NUMLOCK(lOld)

lOld = INSMODE()
lReturned = INSMODE(!lOld)
? "INSMODE returns the state from before the change", lReturned == lOld
INSMODE(lOld)

* READKEY(n): undocumented even in Visual FoxPro's own help, and measured rather than guessed -
* before any editing command has run, READKEY() answers 0 but any call with an argument answers
* 1, whatever the argument itself is. Measured before INKEY touches the buffer below: taking a
* key out of it is not what a READ, GET or editing command ending is, and READKEY() must not
* look like one just because a key passed through.
? "READKEY with no argument", LTRIM(STR(READKEY()))
? "READKEY(1)", LTRIM(STR(READKEY(1)))
? "READKEY(0)", LTRIM(STR(READKEY(0)))

* CHRSAW(nSeconds) and INKEY(nSeconds [, cHideCursor]): nobody is at the keyboard, so a short
* wait ends the same way an empty buffer always does, and a key already queued answers at once.
* LTRIM(STR()) sidesteps a function result's own display width, which is not what a golden is
* measuring here.
? "CHRSAW with nothing queued", CHRSAW(0.1)
KEYBOARD "z"
? "CHRSAW takes the queued key at once", CHRSAW(0.1)
? "INKEY takes the queued key at once", LTRIM(STR(INKEY(0.1)))
KEYBOARD "y"
? "INKEY(nSeconds, cHideCursor) does the same", LTRIM(STR(INKEY(0.1, "H")))
? "INKEY with nothing queued times out to 0", LTRIM(STR(INKEY(0.1)))
