* The three preprocessor directives that ref_sysvars.prg does not reach: the two that read a
* header file beside the program, and the one that is inert in a program a person wrote.
*
* fdvgold.h and fdvmore.h are part of the stage - see tests/fixtures/golden.
#INCLUDE "fdvgold.h"
* #NAME names the object a generated program builds. Nothing generated this one, so it has
* nothing to name and the program runs as though it were not there.
#NAME frmGolden
? FDV_GREETING
? LTRIM(STR(FDV_ANSWER))
? FDV_ON
* a header brings the constants of the header it includes, so they come through two levels
? FDV_NESTED

* #INSERT is the same directive under FoxPro 2.x's name for it, and reading a header twice is
* not an error - the second reading defines the same constants over again
#INSERT fdvmore.h
? FDV_NESTED

* a constant out of a header is a constant like any other: it can be tested for, undefined,
* and asked about again
#IFDEF FDV_ANSWER
? "the header's constant is defined"
#ENDIF
#UNDEF FDV_ANSWER
#IFDEF FDV_ANSWER
? "still defined"
#ELSE
? "gone"
#ENDIF
* and it stands for its text wherever the text would go, not only in an expression
#DEFINE FDV_TIMES 3
cCounted = ""
FOR nEach = 1 TO FDV_TIMES
  cCounted = cCounted + LTRIM(STR(nEach))
NEXT
? cCounted

* COVERS: #INCLUDE, #INSERT, #NAME
