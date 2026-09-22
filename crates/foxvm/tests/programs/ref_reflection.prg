* The functions that ask the runtime about itself and fill an array with the answer, and the
* three that bind one object's event to another object's method.
*
* Each of these is handed the array it fills, and none of the arrays below is declared first:
* Visual FoxPro makes one when the program never did, which is how real code is written.
*
* The awkward calls are in a TRY of their own, and their answer goes into a variable before it
* is printed - `?` writes the newline that ends the open line before it works out what follows,
* so printing an expression that fails leaves a blank line behind.

LOCAL n, oErr, cSaid, oOne, oTwo, oThree, oFour

* --- what class an object is, all the way up to the class it is built on
oOne = CREATEOBJECT("Custom")
n = ACLASS(aWhere, oOne)
? LTRIM(STR(n)), LOWER(aWhere(1))

* --- a class nothing has made an object of: no rows, and the count says so
? LTRIM(STR(AINSTANCE(aNone, "NoSuchClassAnywhere")))

* --- what the runtime has open, which is nothing here but the one data session
? LTRIM(STR(ADATABASES(aDbs))), LTRIM(STR(ASESSIONS(aWhich)))
* nothing is selected in a designer and no form is under the mouse
? LTRIM(STR(ASELOBJ(aPicked)))
* the stack has a row per routine, and how many rows depends on how the program was started;
* how many columns each row has does not
n = ASTACKINFO(aStack)
? LTRIM(STR(ALEN(aStack, 2))), n >= 1

* --- one object's event bound to another object's method
* Each call answers with a count: BINDEVENT with how many delegates that event now has, and
* UNBINDEVENTS with how many bindings it took away.
oTwo = CREATEOBJECT("Custom")
oThree = CREATEOBJECT("Custom")
oFour = CREATEOBJECT("Custom")
? LTRIM(STR(BINDEVENT(oTwo, "Destroy", oThree, "Destroy")))
? LTRIM(STR(BINDEVENT(oTwo, "Destroy", oFour, "Destroy")))
* binding the same pair again changes nothing, and a different event starts again at one
? LTRIM(STR(BINDEVENT(oTwo, "Destroy", oThree, "Destroy")))
? LTRIM(STR(BINDEVENT(oTwo, "Init", oThree, "Destroy")))
* AEVENTS answers 0 either way: a binding between two FoxPro objects is not one of the ones it
* reports, which is not how the reference page reads
? LTRIM(STR(AEVENTS(aBound, 0))), LTRIM(STR(AEVENTS(aBound, 1)))
* raising an event runs whatever is bound to it, and answers a logical
? RAISEEVENT(oTwo, "Init")
? LTRIM(STR(UNBINDEVENTS(oTwo)))
* and unbinding what is no longer bound takes nothing away
? LTRIM(STR(UNBINDEVENTS(oTwo)))
? LTRIM(STR(UNBINDEVENTS(oThree)))

* --- an argument of the wrong type, and a name that is blank
TRY
  n = BINDEVENT(oTwo, "", oThree, "Destroy")
  cSaid = "a blank event name is allowed"
CATCH TO oErr
  cSaid = "blank event name: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid
TRY
  n = BINDEVENT("not an object", "Destroy", oThree, "Destroy")
  cSaid = "a string for the source object is allowed"
CATCH TO oErr
  cSaid = "source not an object: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid
* an object may not handle its own event
TRY
  n = BINDEVENT(oTwo, "Destroy", oTwo, "Destroy")
  cSaid = "an object may handle its own event"
CATCH TO oErr
  cSaid = "bound to itself: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid

* --- asked about a program that is not there
TRY
  cSaid = "file version: " + LTRIM(STR(AGETFILEVERSION(aVersion, "nosuchprogram.exe")))
CATCH TO oErr
  cSaid = "file version: error " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? cSaid

* --- the last error, as an array
TRY
  n = "a name" + 1
CATCH TO oErr
  ? "caught " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
? LTRIM(STR(AERROR(aLast))), LTRIM(STR(ALEN(aLast, 2)))
? LTRIM(STR(aLast(1)))

* COVERS: ACLASS, ADATABASES, AERROR, AEVENTS, AGETFILEVERSION, AINSTANCE, ASELOBJ, ASESSIONS,
* COVERS: ASTACKINFO, BINDEVENT, COMPOBJ, RAISEEVENT, UNBINDEVENTS
