**************************************************
*-- Class Library:  \vfp\ffc\_base.prg
**************************************************





**************************************************
*-- Class:        _column (\vfp\ffc\_base.prg)
*-- ParentClass:  column 
*-- BaseClass:    column 
*
DEFINE CLASS _column AS column


	Name = "_column"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _column
**************************************************





**************************************************
*-- Class:        _cursor (\vfp\ffc\_base.prg)
*-- ParentClass:  cursor
*-- BaseClass:    cursor
*
DEFINE CLASS _cursor AS cursor


	Name = "_cursor"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _cursor
**************************************************





**************************************************
*-- Class:        _dataenvironment (\vfp\ffc\_base.prg)
*-- ParentClass:  dataenvironment
*-- BaseClass:    dataenvironment
*
DEFINE CLASS _dataenvironment AS dataenvironment


	Name = "_dataenvironment"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _dataenvironment
**************************************************





**************************************************
*-- Class:        _header (\vfp\ffc\_base.prg)
*-- ParentClass:  header
*-- BaseClass:    header
*
DEFINE CLASS _header AS header


	Name = "_header"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _header
**************************************************





**************************************************
*-- Class:        _olecontrol (\vfp\ffc\_base.prg)
*-- ParentClass:  olecontrol 
*-- BaseClass:    olecontrol 
*
DEFINE CLASS _olecontrol AS olecontrol


	Name = "_olecontrol"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _olecontrol
**************************************************





**************************************************
*-- Class:        _oleboundcontrol (\vfp\ffc\_base.prg)
*-- ParentClass:  oleboundcontrol 
*-- BaseClass:    oleboundcontrol 
*
DEFINE CLASS _oleboundcontrol AS oleboundcontrol


	Name = "_oleboundcontrol"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _oleboundcontrol
**************************************************





**************************************************
*-- Class:        _optionbutton (\vfp\ffc\_base.prg)
*-- ParentClass:  optionbutton
*-- BaseClass:    optionbutton
*
DEFINE CLASS _optionbutton AS optionbutton


	Name = "_optionbutton"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _optionbutton
**************************************************





**************************************************
*-- Class:        _page (\vfp\ffc\_base.prg)
*-- ParentClass:  page
*-- BaseClass:    page
*
DEFINE CLASS _page AS page


	Name = "_page"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _page
**************************************************





**************************************************
*-- Class:        _relation (\vfp\ffc\_base.prg)
*-- ParentClass:  relation 
*-- BaseClass:    relation 
*
DEFINE CLASS _relation AS relation


	Name = "_relation"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _relation
**************************************************





**************************************************
*-- Class:        _session (\vfp\ffc\_base.prg)
*-- ParentClass:  session
*-- BaseClass:    session
*
DEFINE CLASS _session AS session


	Name = "_session"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _session
**************************************************





**************************************************
*-- Class:        _exception (\vfp\ffc\_base.prg)
*-- ParentClass:  exception 
*-- BaseClass:    exception 
*
DEFINE CLASS _exception AS exception


	Name = "_exception"
	cVersion = ""
	Builder = ""
	BuilderX = (HOME()+"Wizards\BuilderD,BuilderDForm")
	oHost = .NULL.
	vResult = .T.
	cSetObjRefProgram = (IIF(VERSION(2)=0,"",HOME()+"FFC\")+"SetObjRf.prg")
	lAutoBuilder = .F.
	lAutoSetObjectRefs = .F.
	lRelease = .F.
	lIgnoreErrors = .F.
	lSetHost = .F.
	nInstances = 0
	nObjectRefCount = 0
	DIMENSION aObjectRefs[1,3]


	PROCEDURE nInstances_access
	LOCAL laInstances[1]
	
	RETURN AINSTANCE(laInstances,this.Class)
	ENDPROC


	PROCEDURE nInstances_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE release
		IF this.lRelease
			NODEFAULT
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.oHost=.NULL.
		this.ReleaseObjRefs
		RELEASE this
	ENDPROC


	PROCEDURE setobjectref
		LPARAMETERS tcName,tvClass,tvClassLibrary
		LOCAL lvResult

		this.vResult=.T.
		DO (this.cSetObjRefProgram) WITH (this),(tcName),(tvClass),(tvClassLibrary)
		lvResult=this.vResult
		this.vResult=.T.
		RETURN lvResult
	ENDPROC


	PROCEDURE setobjectrefs
		LPARAMETERS toObject

		RETURN
	ENDPROC


	PROCEDURE releaseobjrefs
		LOCAL lcName,oObject,lnCount

		IF this.nObjectRefCount=0
			RETURN
		ENDIF
		FOR lnCount = this.nObjectRefCount TO 1 STEP -1
			lcName=this.aObjectRefs[lnCount,1]
			IF EMPTY(lcName) OR NOT PEMSTATUS(this,lcName,5) OR TYPE("this."+lcName)#"O"
				LOOP
			ENDIF
			oObject=this.&lcName
			IF ISNULL(oObject)
				LOOP
			ENDIF
			IF TYPE("oObject")=="O" AND NOT ISNULL(oObject) AND PEMSTATUS(oObject,"Release",5)
				oObject.Release
			ENDIF
			IF NOT ISNULL(oObject) AND PEMSTATUS(oObject,"oHost",5)
				oObject.oHost=.NULL.
			ENDIF
			this.&lcName=.NULL.
			oObject=.NULL.
		ENDFOR
		DIMENSION this.aObjectRefs[1,3]
		this.aObjectRefs=""
	ENDPROC


	PROCEDURE nobjectrefcount_access
		LOCAL lnObjectRefCount

		lnObjectRefCount=ALEN(this.aObjectRefs,1)
		IF lnObjectRefCount=1 AND EMPTY(this.aObjectRefs[1])
			lnObjectRefCount=0
		ENDIF
		RETURN lnObjectRefCount
	ENDPROC


	PROCEDURE nobjectrefcount_assign
		LPARAMETERS m.vNewVal

		ERROR 1743
	ENDPROC


	PROCEDURE sethost
		this.oHost=IIF(TYPE("thisform")=="O",thisform,.NULL.)
	ENDPROC


	PROCEDURE newinstance
		LPARAMETERS tnDataSessionID
		LOCAL oNewObject,lnLastDataSessionID

		lnLastDataSessionID=SET("DATASESSION")
		IF TYPE("tnDataSessionID")=="N" AND tnDataSessionID>=1
			SET DATASESSION TO tnDataSessionID
		ENDIF
		oNewObject=NEWOBJECT(this.Class,this.ClassLibrary)
		SET DATASESSION TO (lnLastDataSessionID)
		RETURN oNewObject
	ENDPROC


	PROCEDURE Destroy
		IF this.lRelease
			RETURN .F.
		ENDIF
		this.lRelease=.T.
		this.ReleaseObjRefs
		this.oHost=.NULL.
	ENDPROC


	PROCEDURE Init
		IF this.lSetHost
			this.SetHost
		ENDIF
		IF this.lAutoSetObjectRefs AND NOT this.SetObjectRefs(this)
			RETURN .F.
		ENDIF
	ENDPROC


	PROCEDURE Error
		LPARAMETERS nError, cMethod, nLine
		LOCAL lcOnError,lcErrorMsg,lcCodeLineMsg

		IF this.lIgnoreErrors OR _vfp.StartMode>0
			RETURN .F.
		ENDIF
		lcOnError=UPPER(ALLTRIM(ON("ERROR")))
		IF NOT EMPTY(lcOnError)
			lcOnError=STRTRAN(STRTRAN(STRTRAN(lcOnError,"ERROR()","nError"), ;
					"PROGRAM()","cMethod"),"LINENO()","nLine")
			&lcOnError
			RETURN
		ENDIF
		lcErrorMsg=MESSAGE()+CHR(13)+CHR(13)+this.Name+CHR(13)+ ;
				"Error:           "+ALLTRIM(STR(nError))+CHR(13)+ ;
				"Method:       "+LOWER(ALLTRIM(cMethod))
		lcCodeLineMsg=MESSAGE(1)
		IF BETWEEN(nLine,1,100000) AND NOT lcCodeLineMsg="..."
			lcErrorMsg=lcErrorMsg+CHR(13)+"Line:            "+ALLTRIM(STR(nLine))
			IF NOT EMPTY(lcCodeLineMsg)
				lcErrorMsg=lcErrorMsg+CHR(13)+CHR(13)+lcCodeLineMsg
			ENDIF
		ENDIF
		WAIT CLEAR
		MESSAGEBOX(lcErrorMsg,16,_screen.Caption)
		ERROR nError
	ENDPROC


ENDDEFINE
*
*-- EndDefine: _exception
**************************************************
