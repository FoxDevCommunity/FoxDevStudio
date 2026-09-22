* The functions that ask the machine rather than the program: the clock, the keyboard, the
* screen, the code page and the fonts.
*
* What each one answers is the machine's, so what a golden can hold is the shape of the answer -
* its type, its width, the range it must fall in - and not the number itself. That is still
* worth measuring: it is where a wrong type or a wrong unit shows up.

* --- the clock
? VARTYPE(SECONDS()), SECONDS() >= 0, SECONDS() < 86400
? VARTYPE(TIME()), LTRIM(STR(LEN(TIME()))), LTRIM(STR(LEN(TIME(1))))
? SUBSTR(TIME(), 3, 1), SUBSTR(TIME(), 6, 1)
? VARTYPE(DATETIME()), TTOD(DATETIME()) = DATE()
? DATETIME() > {^2000-01-01 00:00:00}

* --- the random numbers, and the two arguments that mean something to them
? VARTYPE(RAND()), RAND() >= 0, RAND() < 1
* a negative seed starts the same sequence every time, so two runs of it agree
? RAND(-1) = RAND(-1)
* and the sequence moves on when nothing is asked of it
? RAND(-1) # RAND()

* --- the keyboard, which nobody is at
? VARTYPE(CAPSLOCK()), VARTYPE(NUMLOCK()), VARTYPE(INSMODE())
? CHRSAW(), LTRIM(STR(INKEY())), LTRIM(STR(LASTKEY())), LTRIM(STR(READKEY()))
? VARTYPE(IMESTATUS())

* --- the screen, the mouse and the pen
? ISCOLOR(), ISMOUSE(), ISPEN()
? VARTYPE(SYSMETRIC(1)), SYSMETRIC(1) > 0, SYSMETRIC(2) > 0
* there are thirty-four measurements and no thirty-fifth, so an index outside them is refused
LOCAL oErr, cSaid
TRY
  cSaid = "SYSMETRIC(35) is " + LTRIM(STR(SYSMETRIC(35)))
CATCH TO oErr
  cSaid = "SYSMETRIC(35): error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid
TRY
  cSaid = "SYSMETRIC(0) is " + LTRIM(STR(SYSMETRIC(0)))
CATCH TO oErr
  cSaid = "SYSMETRIC(0): error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid
? VARTYPE(SYSMETRIC(34))

* --- how much room there is to work in
? VARTYPE(MEMORY()), MEMORY() > 0

* --- the code page, and the conversions between one and another
? VARTYPE(CPCURRENT()), CPCURRENT() > 0
? CPCONVERT(1252, 1252, "plain ascii")
? OEMTOANSI("plain ascii") == "plain ascii"
? ISLEADBYTE("A"), ISLEADBYTE("")

* --- the fonts, measured rather than drawn
? VARTYPE(FONTMETRIC(1, "Arial", 10)), FONTMETRIC(1, "Arial", 10) > 0
* the character cell is what stands above the baseline plus what hangs below it
? FONTMETRIC(2, "Arial", 10) + FONTMETRIC(3, "Arial", 10) = FONTMETRIC(1, "Arial", 10)
* an upright face weighs 400, which is what "regular" means to Windows
? LTRIM(STR(FONTMETRIC(8, "Arial", 10)))
? VARTYPE(WFONT(1)), VARTYPE(WFONT(2)), WFONT(3)

* COVERS: CAPSLOCK, CHRSAW, CPCONVERT, CPCURRENT, DATETIME, FONTMETRIC, IMESTATUS, INKEY,
* COVERS: INSMODE, ISCOLOR, ISLEADBYTE, ISMOUSE, ISPEN, LASTKEY, MEMORY, NUMLOCK, OEMTOANSI,
* COVERS: RAND, READKEY, SECONDS, SYSMETRIC, TIME, WFONT
