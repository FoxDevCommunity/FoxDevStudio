* ERROR raises an error of the program's own, and ON ERROR is put back through a macro
cOld = ON("ERROR")
ON ERROR ? "caught", ERROR(), MESSAGE()
ERROR 1734
ERROR "the widget is missing"
ERROR 12, "no such thing"
ON ERROR &cOld
? ON("ERROR") == cOld
LOCAL lcExpr, lcName
lcExpr = "6 * 7"
? &lcExpr
lcName = "lcExpr"
? &lcName.
* COVERS: ERROR, LOCAL, MESSAGE, ON, ON ERROR
