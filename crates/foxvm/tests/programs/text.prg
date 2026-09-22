#DEFINE GREETING "Hi"
#DEFINE LIMIT 3
? GREETING
? LIMIT * 2
TEXT TO cRaw NOSHOW
line one
  line <<two>>
ENDTEXT
? cRaw
name = "Fox"
n = 2
TEXT TO cMerged TEXTMERGE NOSHOW
Hello <<name>>, <<n + 1>> times
ENDTEXT
? cMerged
TEXT TO cMerged ADDITIVE TEXTMERGE NOSHOW
+more <<name>>
ENDTEXT
? cMerged
TEXT
shown
ENDTEXT
* COVERS: TEXT ... ENDTEXT
