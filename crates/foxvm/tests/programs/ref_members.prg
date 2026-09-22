* AMEMBERS() and GETPEM(): what an object is made of, and what one member holds.
LOCAL o, nProps
o = CREATEOBJECT("Custom")

* nType 0 lists the property names, one column, upper-cased and in alphabetical order
nProps = AMEMBERS(aProps, o)
? ALEN(aProps, 2)
? ASCAN(aProps, "HEIGHT") > 0, ASCAN(aProps, "WIDTH") > 0
* a method is not a property, so it is not in this list
? ASCAN(aProps, "ADDPROPERTY") > 0
? aProps(1) < aProps(2), aProps(2) < aProps(3)

* nType 1 puts the word for what each member is beside it
LOCAL nAll, nRow
nAll = AMEMBERS(aAll, o, 1)
? nAll > nProps
? ALEN(aAll, 2)
nRow = ASUBSCRIPT(aAll, ASCAN(aAll, "HEIGHT"), 1)
? aAll(nRow, 1), aAll(nRow, 2)
nRow = ASUBSCRIPT(aAll, ASCAN(aAll, "ADDPROPERTY"), 1)
? aAll(nRow, 1), aAll(nRow, 2)
nRow = ASUBSCRIPT(aAll, ASCAN(aAll, "INIT"), 1)
? aAll(nRow, 1), aAll(nRow, 2)

* a class answers by name, without one being made
? AMEMBERS(aClass, "Custom", 1) = nAll

* the flags: what the base class declares, and what the program put there for itself
o.AddProperty("nAdded", 7)
? AMEMBERS(aAdded, o, 1, "B")
? aAdded(1, 1), aAdded(1, 2)
? AMEMBERS(aNative, o, 1, "N") = nAll
? AMEMBERS(aOwn, o, 1, "U")
? aOwn(1, 1)
* a call that gives several flags wants a member that answers to any of them
? AMEMBERS(aBoth, o, 1, "NU") = nAll + 1

* a container lists what is in it
LOCAL frm
frm = CREATEOBJECT("Form")
frm.AddObject("cmdOne", "CommandButton")
frm.AddObject("cmdTwo", "CommandButton")
? AMEMBERS(aKids, frm, 2)
? aKids(1), aKids(2)
? ALEN(aKids, 2)

* GETPEM(): the value a member holds now, or the one its class starts it at
? GETPEM(o, "nAdded")
? GETPEM(o, "Height")
? GETPEM("CommandButton", "Height")
? GETPEM("CommandButton", "Width")
? GETPEM("Timer", "Interval")
* a method answers with its source, which a running program is not given
? "[" + GETPEM(o, "AddProperty") + "]"
? VARTYPE(GETPEM(o, "AddProperty"))

* what is not there is a bad argument, not an empty answer
TRY
   ? GETPEM(o, "NotAMember")
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? AMEMBERS(aBad, o, 1, "Z")
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? AMEMBERS(aBad, 5)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY

* COVERS: AMEMBERS, GETPEM
