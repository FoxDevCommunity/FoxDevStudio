* A macro stands for text that is put into the line before the line is read, so it can stand
* for anything a line is made of: a whole clause list, the name of a member, part of a date.
LOCAL lcScope, lnSum, lcEmpty
lcEmpty = ""

CREATE CURSOR nums (n I)
INSERT INTO nums VALUES (1)
INSERT INTO nums VALUES (2)
INSERT INTO nums VALUES (3)

* a macro standing for a whole clause list, and the block it opens going with it
lcScope = "FOR n > 1"
lnSum = 0
SCAN &lcScope
  lnSum = lnSum + nums.n
  * what a string says is left alone, however much of it looks like a macro
  IF lnSum = 2
    ? "&lcScope"
    ? [&lcScope]
  ENDIF
ENDSCAN
? lnSum

* an empty macro stands for nothing at all, which leaves a plain SCAN
lnSum = 0
SCAN &lcEmpty.
  lnSum = lnSum + nums.n
ENDSCAN
? lnSum

* the text of one element of an array
LOCAL ARRAY laText[2]
laText[1] = "40 + 2"
laText[2] = "'two'"
LOCAL i
i = 1
? &laText[m.i]
i = 2
? &laText[m.i]

* a member named by a variable, read and written, inside and outside a WITH
LOCAL cMember
cMember = "Caption"
? oForm.&cMember
WITH oForm
  .&cMember = "renamed"
  ? .&cMember
ENDWITH

* a member named by one element of an array, and then a member of that one
LOCAL ARRAY laObjs[1]
laObjs[1] = "cmdSayHi"
i = 1
? oForm.&laObjs[m.i]..Caption

* a date constant whose parts arrive when the line runs
LOCAL lcYear, lcMonth, lcDay
lcYear = "2024"
lcMonth = "03"
lcDay = "17"
LOCAL ld
ld = {^&lcYear./&lcMonth./&lcDay.}
? DTOC(ld)

* a semicolon inside the text a macro stands for is part of the path, not a line continuation,
* and an empty macro leaves a plain SET PATH TO, which is a line in its own right
LOCAL lcPath
lcPath = "c:\a;c:\b"
SET PATH TO &lcPath.
SET PATH TO &lcEmpty.
? "both paths set"

* what is written inside a string is left alone
LOCAL cName
cName = "not this"
? "&cName"

* a line a macro puts together belongs to the routine that reached it, so a RETURN written in
* one returns from that routine rather than only ending the line
? Stamp()
? "after Stamp"

PROCEDURE Stamp
LOCAL lcY, lcM, lcD
lcY = "2024"
lcM = "12"
lcD = "25"
RETURN DTOC({^&lcY./&lcM./&lcD.})
? "never reached"
ENDPROC

* COVERS: SCAN ... ENDSCAN, WITH ... ENDWITH, LOCAL, CREATE CURSOR, INSERT - SQL, SET PATH, PROCEDURE, RETURN
