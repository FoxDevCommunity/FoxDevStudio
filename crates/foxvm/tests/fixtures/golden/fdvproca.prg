* A procedure file for the set_procedure golden. SET PROCEDURE never runs this line.
? "fdvproca main ran"

PROCEDURE fdvhello
  ? "hello from fdvproca"
ENDPROC

FUNCTION fdvboth
  RETURN "fdvboth in fdvproca"
ENDFUNC

FUNCTION fdvonlya
  * a routine of the same file is found first, whatever else is loaded
  RETURN "fdvonlya calls " + fdvsibling()
ENDFUNC

FUNCTION fdvsibling
  RETURN "its sibling in fdvproca"
ENDFUNC
