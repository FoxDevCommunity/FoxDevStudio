LOCAL a(3), x
a[1] = "a"
a[2] = "b"
a[3] = "c"
?
FOR EACH x IN a
  ?? x
ENDFOR
?
FOR EACH x IN a
  IF x = "b"
    EXIT
  ENDIF
  ?? x
ENDFOR
? x
* COVERS: EXIT, FOR EACH ... ENDFOR, IF ... ENDIF, LOCAL
