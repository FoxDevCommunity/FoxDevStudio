* What a database container itself is told: opened, left, entered, validated, packed and closed,
* and what it hears when its properties and its stored procedures are read or written.
*
* The procedures are built as one string with `|` for a line break, because a TEXT block is
* echoed into the capture even with NOSHOW.
LOCAL cSrc
cSrc = "PROCEDURE dbc_OpenData|LPARAMETERS cDb, lExclusive, lNoUpdate, lValidate|? 'opened ' + LOWER(JUSTFNAME(cDb)) + ' ' + TRANSFORM(lExclusive) + TRANSFORM(lNoUpdate) + TRANSFORM(lValidate)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_Activate|LPARAMETERS cDb|? 'activated ' + LOWER(JUSTFNAME(cDb))|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_Deactivate|LPARAMETERS cDb|? 'deactivated ' + LOWER(JUSTFNAME(cDb))|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_CloseData|LPARAMETERS cDb, lAll|? 'closing ' + LOWER(JUSTFNAME(cDb)) + ' ' + TRANSFORM(lAll)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_PackData|? 'packing'|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_BeforeValidateData|LPARAMETERS lRecover, lNoConsole, lPrint, lFile, cFile|? 'validating ' + TRANSFORM(lRecover) + TRANSFORM(lNoConsole) + TRANSFORM(lPrint) + TRANSFORM(lFile) + ' ' + IIF(lFile, LOWER(JUSTFNAME(cFile)), '-')|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterValidateData|LPARAMETERS lRecover, lNoConsole, lPrint, lFile, cFile|? 'validated'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeCopyProc|LPARAMETERS cFile, nCodePage, lAdditive|? 'copying ' + LOWER(JUSTFNAME(cFile)) + ' ' + LTRIM(STR(nCodePage)) + ' ' + TRANSFORM(lAdditive)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterCopyProc|LPARAMETERS cFile, nCodePage, lAdditive|? 'copied'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeAppendProc|LPARAMETERS cFile, nCodePage, lOverwrite|? 'appending ' + LOWER(JUSTFNAME(cFile)) + ' ' + LTRIM(STR(nCodePage)) + ' ' + TRANSFORM(lOverwrite)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterAppendProc|LPARAMETERS cFile, nCodePage, lOverwrite|? 'appended'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDBGetProp|LPARAMETERS cName, cType, cProperty|? 'getting ' + ALLTRIM(cName) + '.' + ALLTRIM(cProperty) + ' of a ' + ALLTRIM(cType)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterDBGetProp|LPARAMETERS cName, cType, cProperty|? 'got'|"
cSrc = cSrc + "PROCEDURE dbc_BeforeDBSetProp|LPARAMETERS cName, cType, cProperty, eValue|? 'setting ' + ALLTRIM(cName) + '.' + ALLTRIM(cProperty) + ' to ' + TRANSFORM(eValue)|RETURN .T.|"
cSrc = cSrc + "PROCEDURE dbc_AfterDBSetProp|LPARAMETERS cName, cType, cProperty, eValue|? 'set'|"
= STRTOFILE(STRTRAN(cSrc, "|", CHR(13) + CHR(10)), "dataprocs.prg")

CREATE DATABASE depot
APPEND PROCEDURES FROM dataprocs.prg
* the very call that switches the events on is heard by the After event and not the Before one,
* because at the Before the container's events were still off
= DBSETPROP("depot", "Database", "DBCEvents", .T.)

CREATE TABLE crates (code C(4))
USE
= DBSETPROP("crates", "Table", "Comment", "boxes")
? DBGETPROP("crates", "Table", "Comment")

COPY PROCEDURES TO copied.prg
APPEND PROCEDURES FROM copied.prg OVERWRITE

CLOSE DATABASES
OPEN DATABASE depot EXCLUSIVE
VALIDATE DATABASE NOCONSOLE
VALIDATE DATABASE RECOVER NOCONSOLE TO FILE checked.txt
PACK DATABASE

SET DATABASE TO
SET DATABASE TO depot
CLOSE DATABASES ALL

* COVERS: dbc_Activate, dbc_AfterAppendProc, dbc_AfterCopyProc, dbc_AfterDBGetProp,
* COVERS: dbc_AfterDBSetProp, dbc_AfterValidateData, dbc_BeforeAppendProc, dbc_BeforeCopyProc,
* COVERS: dbc_BeforeDBGetProp, dbc_BeforeDBSetProp, dbc_BeforeValidateData, dbc_CloseData,
* COVERS: dbc_Deactivate, dbc_OpenData, dbc_PackData
