* A Visual FoxPro number carries the width it prints in, and `?` right-aligns it inside that
* width. Every line here was run in Visual FoxPro 9 and the `.expected` beside it is what the
* product wrote - `node scripts/vfp-expected.mjs crates/foxvm/tests/programs/ref_numwidth.prg`.
SET DECIMALS TO 2
* a number the program wrote down keeps the characters it was written with
? 8
? 001
? .5
? -7
? 1.50
? 100000
? 0x1F
* a variable's whole part is ten wide, and it keeps the places it was given
n = 4
m = 1.5
k = 1.50
big = 12345678901
? n
? m
? k
? big
* arithmetic works the widths out: `+` takes one more than the wider side, `*` adds them up
* with one for the carry, and `/` gains two places on SET DECIMALS
? n + 1
? n * 2
? n / 2
? n - 1
? n % 3
? n ^ 2
? m * 2
? m + n
? -n
* two numbers the program wrote down are the compiler's work, and it uses different rules
? 4 * 2
? 1.5 * 2
? 100 + 4
? 100 / 4
? 1 / 3
? 4 - 2
? 100 - 4
? 1.5 * 2 / 3
* SET DECIMALS governs division and what the compiler makes of written numbers, and nothing else
SET DECIMALS TO 4
? n / 3
? 1.5 * 2
? m * 2
? n + 1
SET DECIMALS TO 0
? n / 3
? 1.5 * 2
SET DECIMALS TO 2
* SET FIXED gives every number those places, and the room for them
SET FIXED ON
? 8
? n
? n * 2
SET FIXED OFF
? 8
* a field carries the width it was declared with
CREATE CURSOR t (a N(3,0), b N(8,2), c N(5,1), i I, y B(4), mm Y)
APPEND BLANK
REPLACE a WITH 12, b WITH 3200, c WITH 7.5, i WITH 77, y WITH 1.5, mm WITH 12.34
? a
? b
? c
? i
? y
? mm
? a + b
? a * c
? b / c
? b + 1
? a - c
* and it does not follow the value into a variable, only its places do
w = b
? w
* what a function answers is as wide as the function says
? RECNO()
? LEN("abcd")
? ABS(-7)
? ABS(b)
? INT(b)
? INT(3.9)
? ROUND(b, 1)
? SQRT(9)
? MAX(a, c)
? VAL("12")
* a number too wide for the room it has prints in an exponent, or in asterisks
? 10 ^ 20
? 1 / 0
? b / 0
? 10 / (a - a)
* the two print commands put one space between items and none in front of the first
? "xy", n * 2
? 1, 2, 3
?? "z"
?? n
* the width travels through parameters and back out of a function
DO shown WITH 4, w
? doubled(3)

PROCEDURE shown
LPARAMETERS q1, q2
? q1
? q2
? q1 / 2
RETURN

FUNCTION doubled
LPARAMETERS z
RETURN z * 2
