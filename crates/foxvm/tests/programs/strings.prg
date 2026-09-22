? "abc" + "def"
? "ab  " - "cd"
? "b" $ "abc"
? "z" $ "abc"
? "abc" = "ab"
? "ab" = "abc"
? "abc" == "ab"
SET EXACT ON
? "abc" = "ab"
? "abc  " = "abc"
? "abc  " == "abc"
SET EXACT OFF
? "abc" = "ab"
? "a" < "b"
? "Hello" + ", " + "World" + "!"
?? " tail"
? "x"
* COVERS: SET EXACT
