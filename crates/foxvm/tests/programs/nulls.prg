? .NULL.
? 1 + .NULL.
? .NULL. = 1
? .T. AND .NULL.
? .F. AND .NULL.
? .T. OR .NULL.
? .F. OR .NULL.
? NOT .NULL.
x = .NULL.
IF x
  ? "truthy"
ELSE
  ? "null is false"
ENDIF
? IIF(.NULL., "yes", "no")
? "a" + IIF(.T., "b", "c")
* COVERS: IF ... ENDIF, IIF
