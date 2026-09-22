* Every operator the language reference names: what binds tighter than what, what each one does
* to a .NULL., and the places two implementations part company - the modulus of a negative
* number, which way exponentiation associates, the string minus, and the two ways a string
* comparison can be made to mean something else (SET EXACT and ==).
*
* Numbers are printed through STR so the golden says what the arithmetic answered and not how
* wide Visual FoxPro decided to draw it.

* --- numeric, and the order they bind in
? LTRIM(STR(2 + 3 * 4)), LTRIM(STR((2 + 3) * 4))
? LTRIM(STR(20 - 6 / 3)), LTRIM(STR((20 - 6) / 3, 10, 4))
? LTRIM(STR(2 ^ 3 ^ 2)), LTRIM(STR(2 ** 3 ** 2))
? LTRIM(STR(2 ** 3)), LTRIM(STR(2 ^ 0.5, 10, 6))
? LTRIM(STR(-2 ^ 2)), LTRIM(STR((-2) ^ 2))
? LTRIM(STR(2 + 10 % 4)), LTRIM(STR((2 + 10) % 4))

* the modulus of a negative number takes the sign of the divisor, not of the dividend
? LTRIM(STR(10 % 3)), LTRIM(STR(-10 % 3)), LTRIM(STR(10 % -3)), LTRIM(STR(-10 % -3))
? LTRIM(STR(10.5 % 3, 10, 4)), LTRIM(STR(0 % 3))

* --- the two operators a string has
? "ab" + "cd", "ab  " + "cd"
? "[" + ("ab  " - "cd") + "]", "[" + ("ab" - "cd") + "]"
? LTRIM(STR(LEN("ab  " + "cd"))), LTRIM(STR(LEN("ab  " - "cd")))
* a chain of minuses gathers every blank at the very end, once
? "[" + ("a  " - "b  " - "c") + "]"

* --- $ seeks the left string inside the right one, and it is case sensitive
? "b" $ "abc", "B" $ "abc", "abc" $ "abc", "abcd" $ "abc"
? "" $ "abc", "abc" $ "", "" $ ""

* --- = against == , with and without SET EXACT
SET EXACT OFF
? "abc" = "ab", "ab" = "abc", "ab" = "ab "
? "abc" == "ab", "ab " == "ab", "ab" == "ab"
SET EXACT ON
? "abc" = "ab", "ab" = "abc", "ab" = "ab ", "ab " = "ab"
? "ab " == "ab"
SET EXACT OFF

* --- the relational operators, and the three spellings of "not equal"
? 1 < 2, 2 <= 2, 3 > 4, 3 >= 3
? 1 <> 2, 1 # 2, 1 != 2, 2 <> 2
? "a" < "b", {^2024-01-02} > {^2024-01-01}, .F. < .T.

* --- the logical operators, and the order they bind in
? .T. AND .F., .T. OR .F., NOT .T., !.F.
? .T. AND .F. OR .T., .T. OR .F. AND .F.
? NOT .F. AND .F., NOT (.F. AND .F.)
? 2 > 1 AND 3 > 2

* --- what a .NULL. does to each of them
? ISNULL(1 + .NULL.), ISNULL(1 * .NULL.), ISNULL(-.NULL.)
? ISNULL("a" + .NULL.), ISNULL("a" - .NULL.), ISNULL(.NULL. $ "abc")
? ISNULL(1 = .NULL.), ISNULL(1 < .NULL.), ISNULL(.NULL. == .NULL.)
* three-valued logic: an unknown only spreads while it could still change the answer
? ISNULL(.T. AND .NULL.), .F. AND .NULL., ISNULL(.T. OR .NULL.), .T. OR .NULL.
? ISNULL(NOT .NULL.)

* --- a date is a number of days and a datetime a number of seconds
dLeap = {^2024-02-28}
? DTOC(dLeap + 2), DTOC(dLeap - 1)
? LTRIM(STR({^2024-03-01} - {^2024-02-28}))
tEdge = {^2024-02-28 23:59:30}
? TTOC(tEdge + 45)
? LTRIM(STR({^2024-02-28 00:01:00} - {^2024-02-28 00:00:00}))

* --- and where the types do not go together at all
LOCAL oErr, uJunk
TRY
  uJunk = 1 + "a"
CATCH TO oErr
  ? "number + string: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  uJunk = "a" - 1
CATCH TO oErr
  ? "string - number: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  uJunk = {^2024-01-01} + {^2024-01-02}
CATCH TO oErr
  ? "date + date: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  uJunk = 1 < "a"
CATCH TO oErr
  ? "number < string: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  uJunk = .T. + 1
CATCH TO oErr
  ? "logical + number: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  uJunk = 10 % 0
  ? "10 % 0 is " + TRANSFORM(uJunk)
CATCH TO oErr
  ? "10 % 0: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY

* COVERS: -, !=, ( ), *, **, /, #, %, ^, +, <, <=, <>, =, ==, >, >=, $, AND, NOT, OR
