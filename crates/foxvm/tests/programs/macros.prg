cName = "x"
x = 5
? &cName
? &cName + 1
cCmd = "? 'from macro line'"
&cCmd
LOCAL lv
lv = 7
cExpr = "lv * 2"
? &cExpr
cAssign = "lv = 9"
&cAssign
? lv
cNew = "newvar = 'created'"
&cNew
? newvar
* COVERS: LOCAL
