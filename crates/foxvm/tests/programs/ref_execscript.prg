* EXECSCRIPT(): program text compiled and run where the call stands.
LOCAL lcCR
lcCR = CHR(13)

* what the script returns is what the call answers with
? EXECSCRIPT("RETURN 1 + 1")
* a script that returns nothing answers .T., the way any routine that runs off its end does
? EXECSCRIPT("x = 5")
? EXECSCRIPT("")

* parameters, declared either way, and what PCOUNT() then says
? EXECSCRIPT("LPARAMETERS a, b" + lcCR + "RETURN a * b", 6, 7)
? EXECSCRIPT("PARAMETERS a" + lcCR + "RETURN a + 1", 4)
? EXECSCRIPT("LPARAMETERS a, b" + lcCR + "RETURN TRANSFORM(PCOUNT())", 1)

* a script is a whole program: it may declare procedures and classes of its own
? EXECSCRIPT("RETURN f()" + lcCR + "PROCEDURE f" + lcCR + "RETURN 42")
? EXECSCRIPT("LOCAL o" + lcCR + "o = CREATEOBJECT('k')" + lcCR + "RETURN o.nSize" + lcCR + ;
   "DEFINE CLASS k AS Custom" + lcCR + "nSize = 8" + lcCR + "ENDDEFINE")

* the caller's privates are in view, because a private belongs to what the program calls
PRIVATE pcSeen
pcSeen = "from the caller"
? EXECSCRIPT("RETURN pcSeen")

* a LOCAL of the script's own is gone when the script ends
? EXECSCRIPT("LOCAL lnOwn" + lcCR + "lnOwn = 3" + lcCR + "RETURN lnOwn")
? TYPE("lnOwn")

* a script may run one of its own
? EXECSCRIPT("RETURN EXECSCRIPT('RETURN 9')")

* what it prints goes where the caller's output goes
? EXECSCRIPT("? 'printed by the script'")

* text that is not a program is a syntax error, and something that is not text is a bad argument
TRY
   ? EXECSCRIPT("ENDIF")
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? EXECSCRIPT(5)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY

* COVERS: EXECSCRIPT
