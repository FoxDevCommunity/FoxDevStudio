* SCHEME() and RGBSCHEME(): the colour schemes, in letters and in numbers.
* The numbers the product gives come from the running Windows theme, so this asks about the
* shape of the answer and about the letters, which the theme does not move.

* a whole scheme is eleven fields: ten colour pairs and a "+"
? OCCURS(",", SCHEME(1))
? SCHEME(1, 11)
? RGBSCHEME(1, 11)
* leaving the pair out and asking for 0 are the same question
? SCHEME(4) == SCHEME(4, 0)
? RGBSCHEME(4) == RGBSCHEME(4, 0)
? VARTYPE(SCHEME(1)), VARTYPE(RGBSCHEME(1))

* the letters of every scheme, which is the table itself
LOCAL i
FOR i = 1 TO 24
   ? SCHEME(i)
ENDFOR

* a pair on its own is the field at that place in the whole scheme
? SCHEME(3, 7)
? SCHEME(1, 8)
* and the numbers read RGB(fore, back), six of them
? LEFT(RGBSCHEME(1, 8), 4), RIGHT(RGBSCHEME(1, 8), 1)
? OCCURS(",", RGBSCHEME(1, 8))
? RGBSCHEME(1, 8)

* out of range is a bad argument, not an empty answer
TRY
   ? SCHEME(0)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? SCHEME(25)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? SCHEME(1, 12)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? RGBSCHEME(25)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY

* COVERS: RGBSCHEME, SCHEME
