* COVERS: TYPE, VARTYPE
* TYPE(cExpression, 1): the second argument does not add array detection to the plain answer,
* it replaces the plain answer - measured, because the reference page only says the `1` "lets
* you determine" whether cExpression is an array. With it, anything that is not an array reads
* back "U", even a name that evaluates perfectly well on its own.
LOCAL lnVar, loObj
DIMENSION laArr(3)
lnVar = 5
loObj = CREATEOBJECT("Empty")
? "TYPE(array, 1) is A", TYPE("laArr", 1)
? "TYPE(number, 1) is U, not N", TYPE("lnVar", 1)
? "TYPE(object, 1) is U, not O", TYPE("loObj", 1)
? "TYPE(undefined, 1) is U", TYPE("zzz_no_such_var", 1)
? "TYPE(expression, 1) is U", TYPE("1+1", 1)

* VARTYPE(v, lNullDataType): without it, NULL is "X". With it true, a memory variable's own
* NULL - not tied to any column - answers what Visual FoxPro keeps a NULL as internally: "L",
* a Logical with a null bit set. This runtime keeps no declared type of its own for one either,
* so the two agree for the same reason.
LOCAL lnNull
lnNull = .NULL.
? "VARTYPE(NULL)", VARTYPE(lnNull)
? "VARTYPE(NULL, .T.) is L", VARTYPE(lnNull, .T.)
? "VARTYPE(NULL, .F.) is still X", VARTYPE(lnNull, .F.)
? "VARTYPE(number, .T.) is unaffected", VARTYPE(lnVar, .T.)
