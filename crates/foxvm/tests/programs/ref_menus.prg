* A menu built a command at a time, and everything a program can ask about it afterwards.
SET SYSMENU TO
SET MESSAGE TO "Ready"
DEFINE MENU mainbar
DEFINE PAD padfile OF mainbar PROMPT "\<File" KEY ALT+F MESSAGE "Files"
DEFINE PAD padedit OF mainbar PROMPT "\<Edit"
DEFINE PAD padhelp OF mainbar PROMPT "\<Help"
ON PAD padfile OF mainbar ACTIVATE POPUP popfile
ON PAD padedit OF mainbar ACTIVATE POPUP popedit
ON SELECTION PAD padhelp OF mainbar ? "help"

DEFINE POPUP popfile MARGIN SHADOW
DEFINE BAR 1 OF popfile PROMPT "\<New" KEY CTRL+N MESSAGE "A new one"
DEFINE BAR 2 OF popfile PROMPT "\-"
DEFINE BAR 3 OF popfile PROMPT "E\<xit"
ON SELECTION BAR 1 OF popfile ? "new"
ON SELECTION BAR 3 OF popfile ? "exit"

DEFINE POPUP popedit
DEFINE BAR 1 OF popedit PROMPT "Cu\<t"
DEFINE BAR 2 OF popedit PROMPT "\<Copy"
ON SELECTION POPUP popedit ? "edit"
ON EXIT BAR 1 OF popedit ? "left it"

ACTIVATE MENU mainbar NOWAIT

* what is up, and what it holds
? MENU()
? CNTPAD("mainbar")
? CNTBAR("popfile"), BARCOUNT("popedit")
? GETPAD("mainbar", 1), GETPAD("mainbar", 3)
? GETBAR("popfile", 3)
? PRMPAD("mainbar", "padedit")
? PRMBAR("popfile", 1)
? BARPROMPT(3, "popfile")

* the tick beside a choice, and the condition that greys one out
lbusy = .T.
SET MARK OF PAD padedit OF mainbar TO .T.
SET MARK OF BAR 2 OF popfile TO .T.
SET SKIP OF PAD padhelp OF mainbar TO lbusy
SET SKIP OF BAR 1 OF popedit TO NOT lbusy
? MRKPAD("mainbar", "padedit"), MRKPAD("mainbar", "padfile")
? MRKBAR("popfile", 2), MRKBAR("popfile", 1)
? SKPPAD("mainbar", "padhelp"), SKPPAD("mainbar", "padfile")
? SKPBAR("popedit", 1)

* what the last choice was, before anything has been chosen
? BAR(), PAD(), POPUP(), PROMPT()

* kept, changed, and put back
PUSH MENU _MSYSMENU
DEFINE MENU mainbar
? CNTPAD("mainbar")
POP MENU _MSYSMENU
? CNTPAD("mainbar")

PUSH POPUP popfile
DEFINE POPUP popfile
? BARCOUNT("popfile")
POP POPUP popfile
? BARCOUNT("popfile")

* up, hidden, shown and down again
HIDE MENU mainbar
? MENU()
SHOW MENU mainbar
? MENU()
ACTIVATE POPUP popedit
? POPUP()
DEACTIVATE POPUP popedit
HIDE POPUP popedit
DEACTIVATE MENU mainbar
? MENU()

* and released for good
ACTIVATE MENU mainbar
RELEASE POPUP popedit
? BARCOUNT("popedit")
RELEASE MENU mainbar
? CNTPAD("mainbar")
RELEASE MENUS
RELEASE POPUPS

* COVERS: ACTIVATE MENU, ACTIVATE POPUP, BAR, BARCOUNT, BARPROMPT, CNTBAR, CNTPAD,
* COVERS: DEACTIVATE MENU, DEACTIVATE POPUP, DEFINE BAR, DEFINE MENU, DEFINE PAD,
* COVERS: DEFINE POPUP, GETBAR, GETPAD, HIDE MENU, HIDE POPUP, MENU, MRKBAR, MRKPAD,
* COVERS: ON BAR, ON EXIT BAR, ON PAD, ON SELECTION BAR, ON SELECTION PAD,
* COVERS: ON SELECTION POPUP, PAD, POP MENU, POP POPUP, POPUP, PRMBAR, PRMPAD, PROMPT,
* COVERS: PUSH MENU, PUSH POPUP, RELEASE MENUS, RELEASE POPUPS, SET MARK OF, SET MESSAGE,
* COVERS: SET SKIP OF, SET SYSMENU, SHOW MENU, SKPBAR, SKPPAD
