* COVERS: SET
* The second argument of SET(). Only some settings have a second half to report: the two
* characters SET DELIMITERS TO named, whether SET TALK is windowed, how often a view is
* refreshed. Every other setting answers the same however it is asked, which is why asking
* EXCLUSIVE or CARRY for a second answer gives the switch back.
SET TALK OFF
? SET("DELIMITERS")
? SET("DELIMITERS", 1)
? SET("DELIMITERS", 2)
? SET("TALK")
? SET("TALK", 1)
? SET("COMPATIBLE", 1)
? SET("REFRESH", 1)
? SET("TOPIC", 1)
? SET("FIELDS", 1)
? SET("FIELDS", 2)
? SET("EXCLUSIVE", 1)
? SET("CARRY", 1)
? SET("CURSOR", 1)
* the TO form names the second half and leaves the switch where it was
SET DELIMITERS TO "[]"
? SET("DELIMITERS")
? SET("DELIMITERS", 1)
SET DELIMITERS ON
? SET("DELIMITERS")
? SET("DELIMITERS", 1)
