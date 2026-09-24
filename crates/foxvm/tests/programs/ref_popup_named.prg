* COVERS: POPUP
*
* POPUP(cMenuName) asks whether a popup of that name is defined, which is how an application
* that rebuilds its Window menu finds out whether it has one yet. Measured in Visual FoxPro 9.
? "before", TYPE([POPUP("fdvpop")]), POPUP("fdvpop")
DEFINE POPUP fdvpop
DEFINE BAR 1 OF fdvpop PROMPT "one"
? "after", POPUP("fdvpop"), POPUP("FDVPOP")
* the name is not trimmed
? "padded", POPUP(" fdvpop ")
RELEASE POPUPS fdvpop
? "released", POPUP("fdvpop")
* with no argument it is still the name of the popup that is active: none here
? "active", TYPE("POPUP()"), "[" + POPUP() + "]"
