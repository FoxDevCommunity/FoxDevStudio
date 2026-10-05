* COVERS: @
*
* `@m.name` passes the memory variable by reference, as `@name` does: CodeMine's own support
* library writes every by-reference argument that way. Measured in Visual FoxPro 9.
LOCAL lnValue, lcText
lnValue = 1
lcText = "before"
= setboth(@m.lnValue, @m.lcText)
? lnValue, lcText

* and in a call written as a statement's value, through several of them at once
lnValue = 1
? countup(@m.lnValue), lnValue

FUNCTION setboth(tnValue, tcText)
  tnValue = 42
  tcText = "after"
ENDFUNC

FUNCTION countup(tnValue)
  tnValue = tnValue + 1
  RETURN tnValue * 10
ENDFUNC
