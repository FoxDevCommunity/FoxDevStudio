* Money: a 64-bit whole number of ten-thousandths, which is why it is exact where a double is
* not. Every line was run in Visual FoxPro 9 and matched.
SET DECIMALS TO 2
? VARTYPE($1), VARTYPE($1+1), VARTYPE($1+$1), VARTYPE($1*2), VARTYPE($1/$2), VARTYPE(-$1)
? TRANSFORM($1), TRANSFORM($1.5), TRANSFORM($1.23456), TRANSFORM($1/3)
? TRANSFORM($-1.5), TRANSFORM($1000), TRANSFORM($-1234.5)
* exact where a double is not, and the same amount either way through MTON and NTOM
? $0.1 + $0.2 = $0.3, $1 = 1, VARTYPE($1 = 1)
? STR(MTON($1.23456),9,4), STR(MTON($1/3),9,4), STR(MTON($2/3),9,4), STR(MTON($1.5*$1.5),9,4)
? VARTYPE(NTOM(1)), TRANSFORM(NTOM(12.345678))
* the half-way case goes to the even ten-thousandth
? STR(MTON($0.0001/2),9,4), STR(MTON($1.23455),9,4)
* it keeps its type through the functions that answer in the same units, and loses it through
* the ones that do not
? VARTYPE(INT($2.25)), VARTYPE(ABS($2.25)), VARTYPE(CEILING($2.25)), VARTYPE(FLOOR($2.25))
? VARTYPE(ROUND($1.567,1)), VARTYPE(MOD($5,$2)), VARTYPE(MAX($1,$2))
? VARTYPE(SIGN($2.25)), VARTYPE(SQRT($2.25)), VARTYPE(EXP($2.25)), VARTYPE($2 ^ 2)
? TRANSFORM(INT($1.9)), TRANSFORM(ABS($-1.5)), TRANSFORM(ROUND($1.567,1)), TRANSFORM(MOD($5,$2))
? EMPTY($0), STR($12.5,10,2)
* and a table keeps it: a Y field is those same ten-thousandths on disk
CREATE TABLE money FREE (paid Y, note C(6))
INSERT INTO money VALUES ($12.34, "cash")
INSERT INTO money VALUES (CAST(1.5 AS Y), "card")
GO TOP
? VARTYPE(paid), TRANSFORM(paid)
SKIP
? TRANSFORM(paid), TRANSFORM(paid + $0.01)

* COVERS: MTON, NTOM
