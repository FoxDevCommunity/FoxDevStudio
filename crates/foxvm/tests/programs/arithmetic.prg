* arithmetic and precedence
? 1 + 2 * 3
* `-2 ^ 2` and `2 ^ 3 ^ 2` used to be asserted here from memory, and both were asserted wrongly.
* They are in ref_operators.prg now, where Visual FoxPro wrote the answer down.
? 7 % 3
? -7 % 3
? 10 / 4
? 1/3
? (1 + 2) * 3
? 2 * -3
? 10 - 2 - 3
? 100 / 10 / 2
SET DECIMALS TO 4
? 1/3
? 5 > 3 AND 2 > 1
? NOT .T. OR .T.
? 1 = 1 AND 2 <> 2
* COVERS: SET DECIMALS
