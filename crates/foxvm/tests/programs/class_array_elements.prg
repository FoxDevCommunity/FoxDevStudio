* A class body can set every element of an array property, or one element of it
o = CREATEOBJECT("cx")
? o.aAll[1], o.aAll[2], o.aAll[3]
? o.aSome[1], o.aSome[2], o.aSome[3], VARTYPE(o.aSome[3])
? ALEN(o.aSome)
? o.aOrder[1], o.aOrder[2], o.aOrder[3]

DEFINE CLASS cx AS Custom
  DIMENSION aAll[3]
  DIMENSION aSome[3]
  DIMENSION aOrder[3]
  aAll = "x"
  aSome[1] = "one"
  aSome[2] = 2
  aOrder = ""
  aOrder[1] = "WIZARDS"
  aOrder(3) = "APPDIR"
ENDDEFINE
