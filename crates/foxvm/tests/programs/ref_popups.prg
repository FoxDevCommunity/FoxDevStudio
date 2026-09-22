* What a program does to a popup once it has defined one: put it up, move it, resize it, take
* bars and pads out of it again, and hang a command off leaving it.
SET SYSMENU TO
DEFINE POPUP shortcut FROM 2, 4 TO 10, 30 MARGIN SHADOW
DEFINE BAR 1 OF shortcut PROMPT "\<Open"
DEFINE BAR 2 OF shortcut PROMPT "\<Close"
DEFINE BAR 3 OF shortcut PROMPT "\<Rename"
? TRANSFORM(CNTBAR("shortcut"))
? PRMBAR("shortcut", 1), PRMBAR("shortcut", 3)

* a popup shown rather than activated stays up while the program carries on
SHOW POPUP shortcut
MOVE POPUP shortcut TO 3, 6
SIZE POPUP shortcut TO 12, 40
HIDE POPUP shortcut
? TRANSFORM(CNTBAR("shortcut"))

* RELEASE BAR takes one bar out of a popup, or all of them
RELEASE BAR 2 OF shortcut
? TRANSFORM(CNTBAR("shortcut")), PRMBAR("shortcut", 3)
RELEASE BAR ALL OF shortcut
? TRANSFORM(CNTBAR("shortcut"))

* and RELEASE PAD does the same to a menu bar's pads
DEFINE MENU top
DEFINE PAD padone OF top PROMPT "\<One"
DEFINE PAD padtwo OF top PROMPT "\<Two"
DEFINE PAD padall OF top PROMPT "\<Three"
? TRANSFORM(CNTPAD("top"))
RELEASE PAD padtwo OF top
? TRANSFORM(CNTPAD("top")), GETPAD("top", 2)
RELEASE PAD ALL OF top
? TRANSFORM(CNTPAD("top"))

* the commands that run when a menu, a pad or a popup is left rather than chosen from
DEFINE MENU again
DEFINE PAD padx OF again PROMPT "\<X"
DEFINE POPUP more
DEFINE BAR 1 OF more PROMPT "\<Item"
ON EXIT MENU again ? "left the menu"
ON EXIT PAD padx OF again ? "left the pad"
ON EXIT POPUP more ? "left the popup"
ON SELECTION MENU again ? "chose from the menu"
? TRANSFORM(CNTPAD("again")), TRANSFORM(CNTBAR("more"))
RELEASE POPUPS more
RELEASE MENUS again
RELEASE POPUPS shortcut
RELEASE MENUS top
? MENU(), POPUP()
* COVERS: HIDE POPUP, MOVE POPUP, ON EXIT MENU, ON EXIT PAD, ON EXIT POPUP,
* COVERS: ON SELECTION MENU, RELEASE BAR, RELEASE PAD, SHOW POPUP, SIZE POPUP
