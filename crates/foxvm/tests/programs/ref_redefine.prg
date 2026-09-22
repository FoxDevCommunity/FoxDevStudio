* Assigning to the word that names what a method is running in.
*
* `THISFORM = .NULL.` compiles in Visual FoxPro - written out, run through COMPILE, and no .err
* file appears beside the .fxp - and refuses only when the line is reached. The builder library
* Visual FoxPro ships writes exactly that, so refusing it at compile time took the whole method
* with it and every other line in it stopped working.
? "before"
TRY
   THISFORM = .NULL.
CATCH TO oErr
   ? "error " + TRANSFORM(oErr.ErrorNo) + " " + oErr.Message
ENDTRY
TRY
   THIS = .NULL.
CATCH TO oErr2
   ? "error " + TRANSFORM(oErr2.ErrorNo) + " " + oErr2.Message
ENDTRY
? "after"
* COVERS: =
