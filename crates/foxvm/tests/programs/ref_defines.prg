* A constant can stand for other constants, and its value can be continued onto the next line.
* The OLE tree sample does both at once, and reached the program as a name without this.
#DEFINE FIELD1_LOC "KEY"
#DEFINE FIELD2_LOC "PARENT"
#DEFINE MSG_LOC "fields:" + CHR(13) + ;
   CHR(13) + FIELD1_LOC + ;
   CHR(13) + FIELD2_LOC
? STRTRAN(MSG_LOC, CHR(13), "|")
* and one defined inside a block is in force from there on
IF .T.
  #DEFINE TITLE_LOC "Invalid Table Structure"
  ? TITLE_LOC
ENDIF

* COVERS: #DEFINE
