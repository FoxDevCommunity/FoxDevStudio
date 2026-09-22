* COVERS: BINDEVENT, UNBINDEVENTS
* BINDEVENT's fifth argument, nFlags, and the three UNBINDEVENTS forms no other golden calls:
* none at all, two arguments and three.

LOCAL oErr
oOne = CREATEOBJECT("Custom")
oTwo = CREATEOBJECT("Custom")
oThree = CREATEOBJECT("Custom")

* --- BINDEVENT(5): nFlags 1 asks for the delegate to run before the event itself; either way,
* the answer is how many delegates the event now has
? "BINDEVENT(4) first delegate", BINDEVENT(oOne, "Init", oTwo, "Destroy")
? "BINDEVENT(5) second delegate", BINDEVENT(oOne, "Init", oThree, "Destroy", 1)
? "----"

* --- UNBINDEVENTS(4): the full binding, named the way BINDEVENT() named it
? "UNBINDEVENTS(4)", UNBINDEVENTS(oOne, "Init", oTwo, "Destroy")

* --- UNBINDEVENTS(2) and UNBINDEVENTS(3): neither is one of the two shapes the product
* recognises - the single object, or all four arguments - and both are error 11, the same as any
* other bad argument count this runtime's own arity check cannot tell from a good one
TRY
  = UNBINDEVENTS(oOne, "Init")
CATCH TO oErr
  ? "UNBINDEVENTS(2) errors", oErr.ErrorNo
ENDTRY
TRY
  = UNBINDEVENTS(oOne, "Init", oThree)
CATCH TO oErr
  ? "UNBINDEVENTS(3) errors", oErr.ErrorNo
ENDTRY
? "----"

* --- UNBINDEVENTS(0): no arguments at all is "too few", not a wildcard that drops every binding
TRY
  = UNBINDEVENTS()
CATCH TO oErr
  ? "UNBINDEVENTS(0) errors", oErr.ErrorNo, oErr.Message
ENDTRY

* --- the binding UNBINDEVENTS(2) and (3) left alone is still there, so cleaning it up with the
* one-argument form still finds it
? "cleanup", UNBINDEVENTS(oOne)
