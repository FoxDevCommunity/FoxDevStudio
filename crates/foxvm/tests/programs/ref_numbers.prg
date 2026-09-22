* What a number is: an IEEE double whose result is rounded to fifteen significant digits, which
* is where a double's exactness runs out. Every line was run in Visual FoxPro 9 and matched.
SET DECIMALS TO 2
? 0.1 + 0.2 = 0.3
? 0.1 * 3 = 0.3
? 1/7 * 7 = 1
? STR(0.1 + 0.2, 22, 18)
? STR(1/3, 20, 16)
? STR(2/3, 20, 16)
? STR(0.1, 20, 17)
* and it is still a double underneath: the gap at 2^53 is where integers stop being exact
? STR(1e15 + 1 - 1e15, 20, 0)
? STR(1e16 + 1 - 1e16, 20, 0)
? STR(2^53 + 1, 20, 0)
* a literal keeps what was written, including a sixteenth digit
? STR(123456789012345.6, 20, 1)

* COVERS: STR
