* Clauses and declarations real Visual FoxPro takes and this runtime used to turn down.
LOCAL ARRAY laCountries[3]
laCountries[1] = "Chile"
laCountries[2] = "Peru"
laCountries[3] = "Brazil"

* the loop variable may say it is a memory variable, and FOXOBJECT and AS are clauses on the
* statement rather than part of what is walked
?
FOR EACH m.cCountry IN laCountries
  ?? m.cCountry + " "
ENDFOR
?
FOR EACH m.cCountry IN laCountries FOXOBJECT
  ?? m.cCountry + " "
ENDFOR
?
FOR EACH m.cCountry IN laCountries AS String FOXOBJECT
  ?? m.cCountry + " "
ENDFOR

* the caught error may say it is a memory variable too
TRY
  ERROR "caught"
CATCH TO m.oError
  ? m.oError.Message
ENDTRY

* PRIVATE ARRAY hides an array the caller owns. Unlike LOCAL and PUBLIC it needs no size,
* because it creates nothing: something else has to DIMENSION the array.
PRIVATE ARRAY paDBFields
PRIVATE ARRAY paSized[4]
DIMENSION paDBFields[2]
paDBFields[1] = "hidden"
? paDBFields[1]

* `=>` and `=<` are `>=` and `<=` written the other way round
? IIF(5 => 5, "5=>5", "no")
? IIF(5 =< 3, "yes", "5=<3 is false")

* COVERS: FOR EACH ... ENDFOR, TRY...CATCH...FINALLY, PRIVATE, DIMENSION, LOCAL
