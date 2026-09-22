* The menu functions' argument forms a golden had not yet called: BARCOUNT, CNTBAR and CNTPAD
* asked with no menu named, and BARPROMPT asked for a bar with no popup named.
*
* Leaving the name out means "the active menu or popup", and this probe defines one without
* making it the active one - ACTIVATE MENU/POPUP needs a person at the keyboard to really take
* over, which nothing here can be. With none active, BARCOUNT, CNTBAR and CNTPAD count an
* unresolved menu as having nothing in it and answer 0; BARPROMPT has to find the popup itself
* before it can find a bar of it, and refuses instead.
DEFINE MENU mbar
DEFINE PAD padone OF mbar PROMPT "\<One"
DEFINE PAD padtwo OF mbar PROMPT "\<Two"

DEFINE POPUP pop1
DEFINE BAR 1 OF pop1 PROMPT "\<A"
DEFINE BAR 2 OF pop1 PROMPT "\<B"

? CNTPAD()
? CNTBAR()
? BARCOUNT()
TRY
  ? BARPROMPT(1)
CATCH TO oErr
  ? "caught", TRANSFORM(oErr.ErrorNo)
ENDTRY

RELEASE MENUS
RELEASE POPUPS

* COVERS: BARCOUNT, BARPROMPT, CNTBAR, CNTPAD
