* COVERS: CAST
* CAST always takes its type through `AS`, never through a comma the way an ordinary function
* would: `CAST(eExpression, cType)` is not a second calling convention for the same thing, it is
* a syntax error - measured, because this runtime used to accept it silently as if it were one.
? CAST(3, "N")
