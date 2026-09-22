* SetObjRf.PRG - Set Object Referece.
*
* Copyright (c) 1997 Microsoft Corp.
* 1 Microsoft Way
* Redmond, WA 98052
*
* Description:
* Set an object reference to a specified property based on a specified class.
* Return new instance of specified class if name is an empty string.

LPARAMETERS toObject,tcName,tvClass,tvClassLibrary
LOCAL lcName,lcClass,lcClassLibrary,oObject,lnCount
LOCAL lnObjectRefIndex,lnObjectRefCount,oExistingObject

IF TYPE("toObject")#"O" OR ISNULL(toObject)
	RETURN .NULL.
ENDIF
lcName=IIF(TYPE("tcName")=="C",ALLTRIM(tcName),LOWER(SYS(2015)))
oExistingObject=.NULL.
oObject=.NULL.
lcClassLibrary=""
DO CASE
	CASE TYPE("tvClass")=="O"
		oObject=tvClass
		lcClass=LOWER(oObject.Class)
		lcClassLibrary=LOWER(oObject.ClassLibrary)
		IF NOT ISNULL(oExistingObject) AND LOWER(oExistingObject.Class)==lcClass AND ;
				LOWER(oExistingObject.ClassLibrary)==lcClassLibrary
			toObject.vResult=oExistingObject
			RETURN toObject.vResult
		ENDIF
	CASE EMPTY(tvClass)
		oObject=toObject
		lcClass=LOWER(oObject.Class)
		lcClassLibrary=LOWER(oObject.ClassLibrary)
		IF NOT ISNULL(oExistingObject) AND LOWER(oExistingObject.Class)==lcClass AND ;
				LOWER(oExistingObject.ClassLibrary)==lcClassLibrary
			toObject.vResult=oExistingObject
			RETURN toObject.vResult
		ENDIF
	OTHERWISE
		lcClass=LOWER(ALLTRIM(tvClass))
		DO CASE
			CASE TYPE("tvClassLibrary")=="O"
				lcClassLibrary=LOWER(tvClassLibrary.ClassLibrary)
			CASE TYPE("tvClassLibrary")=="C"
				IF EMPTY(tvClassLibrary)
					lcClassLibrary=LOWER(toObject.ClassLibrary)
				ELSE
					lcClassLibrary=LOWER(ALLTRIM(tvClassLibrary))
					IF EMPTY(JUSTEXT(lcClassLibrary))
						lcClassLibrary=LOWER(FORCEEXT(lcClassLibrary,"vcx"))
					ENDIF
					llClassLib=(JUSTEXT(lcClassLibrary)=="vcx")
					IF NOT "\"$lcClassLibrary
						lcClassLibrary=LOWER(FORCEPATH(lcClassLibrary,JUSTPATH(toObject.ClassLibrary)))
						IF NOT FILE(lcClassLibrary) AND VERSION(2)#0
							lcClassLibrary=LOWER(FORCEPATH(lcClassLibrary,HOME()+"ffc\"))
							IF NOT FILE(lcClassLibrary)
								lcClassLibrary=LOWER(FULLPATH(JUSTFNAME(lcClassLibrary)))
							ENDIF
						ENDIF
					ENDIF
					IF NOT FILE(lcClassLibrary)
						toObject.vResult=.NULL.
						RETURN toObject.vResult
					ENDIF
				ENDIF
			OTHERWISE
				lcClassLibrary=""
		ENDCASE
		IF NOT ISNULL(oExistingObject) AND LOWER(oExistingObject.Class)==lcClass AND ;
				LOWER(oExistingObject.ClassLibrary)==lcClassLibrary
			toObject.vResult=oExistingObject
			RETURN toObject.vResult
		ENDIF
		oObject=NEWOBJECT(lcClass,lcClassLibrary)
		IF TYPE("oObject")#"O" OR ISNULL(oObject)
			toObject.vResult=.NULL.
			RETURN toObject.vResult
		ENDIF
ENDCASE
DO CASE
	CASE EMPTY(lcName)
		toObject.vResult=oObject
		RETURN toObject.vResult
	OTHERWISE
		IF NOT toObject.AddProperty(lcName,oObject)
			oObject=.NULL.
		ENDIF
ENDCASE
IF ISNULL(oObject)
	toObject.vResult=.NULL.
	RETURN toObject.vResult
ENDIF
IF PEMSTATUS(oObject,"oHost",5)
	oObject.oHost=toObject.oHost
ELSE
	oObject.AddProperty("oHost",toObject.oHost)
ENDIF
IF EMPTY(lcClassLibrary)
	lcClassLibrary=LOWER(oObject.ClassLibrary)
ENDIF
lnObjectRefCount=toObject.nObjectRefCount
lnObjectRefIndex=lnObjectRefCount+1
FOR lnCount = 1 TO lnObjectRefCount
	IF toObject.aObjectRefs[lnCount,1]==LOWER(lcName)
		lnObjectRefIndex=lnCount
		EXIT
	ENDIF
ENDFOR
IF lnObjectRefIndex>lnObjectRefCount
	DIMENSION toObject.aObjectRefs[lnObjectRefIndex,3]
ENDIF
toObject.aObjectRefs[lnObjectRefIndex,1]=LOWER(lcName)
toObject.aObjectRefs[lnObjectRefIndex,2]=lcClass
toObject.aObjectRefs[lnObjectRefIndex,3]=lcClassLibrary
toObject.vResult=oObject
RETURN toObject.vResult
