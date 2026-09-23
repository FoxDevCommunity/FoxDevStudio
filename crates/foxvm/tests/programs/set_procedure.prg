* COVERS: SET PROCEDURE, RELEASE PROCEDURE, SET, DO
*
* Where a bare call finds its routine: the program it is written in, then the procedure files
* in the order SET PROCEDURE listed them, then the programs that have run. Every answer below was
* measured in Visual FoxPro 9. SET("PROCEDURE") is printed as the files' stems, because what it
* really answers is the full path of each .FXP, and that is wherever the program ran.
? "none" + " " + procs()
SET PROCEDURE TO fdvproca
? "one" + " " + procs()
? "extension" + " " + JUSTEXT(CHRTRAN(SET("PROCEDURE"), '"', ""))

* the file's own first line never runs; its routines are found by DO and by a call
DO fdvhello
? fdvboth()
? fdvonlya()

* ADDITIVE puts a file on the end, and the first file listed wins a name both have; the
* program's own routine beats every procedure file
SET PROCEDURE TO fdvprocb ADDITIVE
? "additive" + " " + procs()
? fdvboth()
? fdvonlyb()
? fdvmine()

* without ADDITIVE the list is replaced, and a routine of a file that went is not found
SET PROCEDURE TO fdvprocb
? "replaced" + " " + procs()
? fdvboth()
TRY
  lcGot = fdvhello()
CATCH TO loErr
  ? "gone" + " " + TRANSFORM(loErr.ErrorNo) + " " + loErr.Message
ENDTRY

* several at once, in the order written
SET PROCEDURE TO fdvproca, fdvprocb
? "two" + " " + procs() + " " + fdvboth()
SET PROCEDURE TO fdvprocb, fdvproca
? "other order" + " " + procs() + " " + fdvboth()

* a file already listed stays where it is
SET PROCEDURE TO fdvproca ADDITIVE
? "again" + " " + procs()

* RELEASE PROCEDURE takes one off; SET PROCEDURE TO on its own empties the list
RELEASE PROCEDURE fdvprocb
? "released" + " " + procs() + " " + fdvboth()
SET PROCEDURE TO
? "emptied" + " " + procs()
TRY
  lcGot = fdvboth()
CATCH TO loErr
  ? "not found" + " " + TRANSFORM(loErr.ErrorNo) + " " + loErr.Message
ENDTRY

* a file that is not there is error 1; without ADDITIVE the list is emptied first, so what was
* on it is gone, and a file listed before the missing one is on it
SET PROCEDURE TO fdvproca
TRY
  SET PROCEDURE TO fdvnosuch
CATCH TO loErr
  ? "missing" + " " + TRANSFORM(loErr.ErrorNo) + " " + loErr.Message
ENDTRY
? "after missing" + " " + procs()
TRY
  SET PROCEDURE TO fdvproca, fdvnosuch, fdvprocb
CATCH TO loErr
  ? "missing second" + " " + TRANSFORM(loErr.ErrorNo) + " " + loErr.Message
ENDTRY
? "after missing second" + " " + procs()
TRY
  SET PROCEDURE TO fdvnosuch ADDITIVE
CATCH TO loErr
  ? "missing additive" + " " + TRANSFORM(loErr.ErrorNo) + " " + loErr.Message
ENDTRY
? "after missing additive" + " " + procs()

* the name may be worked out
lcLib = "fdvprocb"
SET PROCEDURE TO (lcLib)
? "expression" + " " + procs()
SET PROCEDURE TO &lcLib ADDITIVE
? "macro" + " " + procs()
SET PROCEDURE TO fdvproca.prg
? "with extension" + " " + procs()

* a program that ran with DO keeps its routines findable after it returns, and a program's
* name may be built right against a `+`
SET PROCEDURE TO
lcPre = "fdv"
DO lcPre+"runme"
? fdvinrunme()
RETURN

FUNCTION fdvmine
  RETURN "fdvmine in main"
ENDFUNC

* the stems of the files on the list, in order
FUNCTION procs
  LOCAL lcList, lcOut, lnCount, lnI
  lcList = SET("PROCEDURE")
  IF EMPTY(lcList)
    RETURN "[]"
  ENDIF
  lcOut = ""
  lnCount = GETWORDCOUNT(lcList, ",")
  FOR lnI = 1 TO lnCount
    lcOut = lcOut + IIF(lnI > 1, " ", "") + JUSTSTEM(CHRTRAN(ALLTRIM(GETWORDNUM(lcList, lnI, ",")), '"', ""))
  ENDFOR
  RETURN "[" + lcOut + "]"
ENDFUNC
