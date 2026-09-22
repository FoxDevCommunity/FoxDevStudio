* An array property, sized. Brackets and parentheses index alike in FoxPro, so both spellings
* are the same statement - the foundation classes write DIME THIS.aMemos(THIS.iMemos), which
* reads as a method call until the DIMENSION says otherwise.
PUBLIC oHolder
oHolder = CREATEOBJECT("Custom")
oHolder.AddProperty("aRows[1]")
DIMENSION oHolder.aRows[3]
? ALEN(oHolder.aRows)
DIME oHolder.aRows(5)
? ALEN(oHolder.aRows)
WITH oHolder
  DIMENSION .aRows[2, 2]
  ? ALEN(.aRows), ALEN(.aRows, 1), ALEN(.aRows, 2)
ENDWITH

* Sizing a property is not the same as giving it a fresh array: what still fits is kept, which
* is what a form's Init leans on when it dimensions the array its class declared and then fills
* it in. Growing keeps the elements where they were, row by row; shrinking keeps the front.
oHolder.aRows[1, 1] = "one"
oHolder.aRows[2, 2] = "four"
DIMENSION oHolder.aRows[4, 2]
? oHolder.aRows[1, 1], oHolder.aRows[2, 2], ALEN(oHolder.aRows)
DIMENSION oHolder.aRows[1, 1]
? oHolder.aRows[1, 1], ALEN(oHolder.aRows)

* ADDPROPERTY() writes the subscripts too, and the value it is given goes in every element
oHolder.AddProperty("aFilled[3]", 5)
? TRANSFORM(oHolder.aFilled[1]), TRANSFORM(oHolder.aFilled[2]), TRANSFORM(oHolder.aFilled[3])
oHolder.AddProperty("aTable[2, 3]")
? ALEN(oHolder.aTable), ALEN(oHolder.aTable, 1), ALEN(oHolder.aTable, 2)

* what it refuses: a property that is not an array, and one the object has never heard of
oHolder.AddProperty("cName", "plain")
TRY
  DIMENSION oHolder.cName[2]
CATCH TO oErr
  ? TRANSFORM(oErr.ErrorNo), oErr.Message
ENDTRY
TRY
  DIMENSION oHolder.aNothing[2]
CATCH TO oErr2
  ? TRANSFORM(oErr2.ErrorNo), oErr2.Message
ENDTRY

* COVERS: DIMENSION
