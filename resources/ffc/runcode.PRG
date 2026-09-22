* RunCode.PRG - Run code block interpreter.
* Version 2.00
*
* Copyright (c) 1998-1999 Microsoft Corp.
* 1 Microsoft Way
* Redmond, WA 98052
*
* Description:
* Runs a block of VFP code via macros without compilation.
*
* Parameter list:
* cCode:	Code to execute or file name that contains code to execute
* lFile:	Specifies source file mode.
*				.F./Empty = Code is specified by cCode.
*				.T. = Code is imported from specified file via cCode value.
* lIgnoreErrors:	Specifies error handling mode.
*				.F./Empty = Errors are trapped and displayed in a wait window.
*				.T. = All errors are ignored.


LPARAMETERS __tcCode,__tlFile,__tvIgnoreErrors
LOCAL __lcCode,__lcOnError,__llArrayCode,__lcLine,__lnLine,__lcLine2,__llVFP60
LOCAL __lcCommand,__lcExpr,__lcChar,__lnAtPos,__lnAtPos2,__lnOccurrence
LOCAL __lnLineTotal,__llTextMode,__lcLastOnError,__lvResult
LOCAL __lcDoExpr,__lnDoLine,__lnDoLineTotal,__lnDoStackCount
LOCAL __lcForExpr,__lnForMax,__lnForStep,__lnForLine,__lnForLineTotal,__lnForStackCount
LOCAL __lcIfExpr,__llIfExpr,__lnIfLine,__lnIfLineTotal,__lnIfStackCount
LOCAL __laLines[1],__laForLines[1],__laIfLines[1],__laDoLines[1]
EXTERNAL ARRAY __tcCode,__laLines,__laForLines,__laIfLines,__laDoLines

#DEFINE TAB		CHR(9)
#DEFINE LF		CHR(10)
#DEFINE CR		CHR(13)
#DEFINE CR_LF	CR+LF

IF VARTYPE(__tvIgnoreErrors)=="C"
	__lcOnError=ALLTRIM(__tvIgnoreErrors)
ELSE
	__lcOnError=IIF(__tvIgnoreErrors,"=.F.","__")
ENDIF
__llVFP60=" 06.0"$VERSION()
IF NOT __llVFP60
	__lcLastOnError=ON("ERROR")
	DO CASE
		CASE __lcOnError=="__"
			ON ERROR __RunCodeError(ERROR(),0,"RunCode","",MESSAGE())
		CASE __lcOnError=="=.F."
			ON ERROR =.F.
		CASE EMPTY(__lcOnError)
			ON ERROR
		OTHERWISE
			ON ERROR &__lcOnError
	ENDCASE
	__lvResult=.T.
	IF __tlFile
		__lcCode=""
		__lcCode=ALLTRIM(FILETOSTR(__tcCode))
	ELSE
		__lcCode=IIF(VARTYPE(__tcCode)=="C",__tcCode,"")
	ENDIF
	IF LEFT(__lcCode,1)==";"
		__lcCode=STRTRAN(__lcCode,";",CR_LF)
	ENDIF
	__lvResult=EVALUATE("ExecScript(__lcCode)")
	IF EMPTY(__lcLastOnError)
		ON ERROR
	ELSE
		ON ERROR &__lcLastOnError
	ENDIF
	RETURN __lvResult
ENDIF
__llArrayCode=(TYPE("__tcCode[1]")=="C")
IF __llArrayCode
	__lnLineTotal=ACOPY(__tcCode,__laLines)
ELSE
	IF VARTYPE(__tcCode)#"C" OR EMPTY(__tcCode)
		RETURN
	ENDIF
	IF __tlFile
		__lcCode=""
		__lcCode=ALLTRIM(FILETOSTR(__tcCode))
		IF EMPTY(__lcCode)
			RETURN
		ENDIF
	ELSE
		__lcCode=ALLTRIM(__tcCode)
	ENDIF
	IF LEFT(__lcCode,1)==";"
		__lcCode=STRTRAN(__lcCode,";",CR_LF)
	ENDIF
	__lnLineTotal=ALINES(__laLines,__lcCode)
	IF __lnLineTotal=0
		RETURN
	ENDIF
	PRIVATE __lcLastLine
	__lcLastLine=""
	__lnLine=0
	DO WHILE __lnLine<__lnLineTotal
		__lnLine=__lnLine+1
		__lcLine=ALLTRIM(__laLines[__lnLine])
		__lnAtPos=AT("&"+"&",__lcLine)
		IF __lnAtPos>0
			__lcLine=ALLTRIM(LEFT(__lcLine,__lnAtPos-1))
		ENDIF
		DO WHILE .T.
			__lcChar=LEFT(__lcLine,1)
			IF __lcChar==" " OR __lcChar==TAB
				__lcLine=ALLTRIM(SUBSTR(__lcLine,2))
				LOOP
			ENDIF
			__lcChar=RIGHT(__lcLine,1)
			IF __lcChar==" " OR __lcChar==TAB
				__lcLine=TRIM(LEFT(__lcLine,LEN(__lcLine)-1))
				LOOP
			ENDIF
			EXIT
		ENDDO
		IF EMPTY(__lcLine) OR LEFT(__lcLine,1)=="*" OR LEFT(__lcLine,1)=="#" OR ;
				LEFT(__lcLine,2)==("&"+"&") OR UPPER(LEFT(__lcLine,4))=="NOTE" OR ;
				LEFT(__lcLine,4)=="<!--"
			ADEL(__laLines,__lnLine)
			__lnLineTotal=__lnLineTotal-1
			DIMENSION __laLines[__lnLineTotal]
			__lnLine=__lnLine-1
			LOOP
		ENDIF
		IF __lnLine>=2 AND RIGHT(__laLines[__lnLine-1],1)==";"
			__lcLine2=LEFT(__laLines[__lnLine-1],LEN(__laLines[__lnLine-1])-1)
			DO WHILE .T.
				__lcChar=RIGHT(__lcLine2,1)
				IF __lcChar==" " OR __lcChar==TAB
					__lcLine2=TRIM(LEFT(__lcLine2,LEN(__lcLine2)-1))
					LOOP
				ENDIF
				EXIT
			ENDDO
			__lnLine=__lnLine-1
			__lcLine=__lcLine2+" "+__lcLine
			ADEL(__laLines,__lnLine)
			__lnLineTotal=__lnLineTotal-1
			DIMENSION __laLines[__lnLineTotal]
			__laLines[__lnLine]=__lcLine
		ELSE
			__laLines[__lnLine]=__lcLine
		ENDIF
	ENDDO
ENDIF
IF __lnLineTotal=0
	RETURN
ENDIF
__lcLastOnError=ON("ERROR")
DO CASE
	CASE __lcOnError=="__"
		ON ERROR __RunCodeError(ERROR(),0,"RunCode",__lcLastLine,MESSAGE())
	CASE __lcOnError=="=.F."
		ON ERROR =.F.
	CASE EMPTY(__lcOnError)
		ON ERROR
	OTHERWISE
		ON ERROR &__lcOnError
ENDCASE
__lvResult=.T.
__lcLine=""
STORE .F. TO __llIfExpr,__llTextMode
STORE "" TO __lcDoExpr,__lcForExpr,__lcIfExpr
STORE 0 TO __lnLine,__lnDoLine,__lnDoLineTotal,__lnDoStackCount, ;
		__lnForLine,__lnForLineTotal,__lnForStackCount,__lnForMax, ;
		__lnForStep,__lnIfLine,__lnIfLineTotal,__lnIfStackCount
DO WHILE __lnLine<__lnLineTotal
	__lnLine=__lnLine+1
	__lcLine=__laLines[__lnLine]
	IF EMPTY(__lcLine)
		LOOP
	ENDIF
	IF LEFT(__lcLine,1)=="="
		EVALUATE(SUBSTR(__lcLine,2))
		LOOP
	ENDIF
	__lcCommand=UPPER(LEFT(__lcLine,4))
	IF __lcCommand=="DO W" AND (UPPER(LEFT(__lcLine,8))=="DO WHIL " OR ;
			UPPER(LEFT(__lcLine,8))=="DO WHILE")
		__lcCommand="DO_W"
		__lnOccurrence=2
	ELSE
		__lnOccurrence=1
	ENDIF
	__lnAtPos=AT(" ",__lcCommand,__lnOccurrence)
	__lnAtPos2=AT(TAB,__lcCommand,__lnOccurrence)
	IF BETWEEN(__lnAtPos2,1,__lnAtPos)
		__lnAtPos=__lnAtPos2
	ENDIF
	IF __lnAtPos>0
		__lcCommand=LEFT(__lcCommand,__lnAtPos-1)
	ENDIF
	__lnAtPos=AT(" ",__lcLine,__lnOccurrence)
	__lnAtPos2=AT(TAB,__lcLine,__lnOccurrence)
	IF BETWEEN(__lnAtPos2,1,__lnAtPos)
		__lnAtPos=__lnAtPos2
	ENDIF
	IF __lnAtPos=0
		__lcExpr=""
	ELSE
		__lcExpr=ALLTRIM(SUBSTR(__lcLine,__lnAtPos+1))
	ENDIF
	__lcLastLine=__lcLine
	DO CASE
		CASE __lcCommand=="ENDT"
			__llTextMode=.F.
			LOOP
		CASE __llTextMode
			__lcLine="\"+__lcLine
			__lcLastLine=__lcLine
			&__lcLine
			LOOP
		CASE __lcCommand=="DO_W"
			IF __lnForStackCount<=0 AND __lnIfStackCount<=0
				__lnDoStackCount=__lnDoStackCount+1
				IF __lnDoStackCount<=1
					__lcDoExpr=__lcExpr
					__lnDoLine=__lnLine
					LOOP
				ENDIF
			ENDIF
		CASE __lcCommand=="FOR"
			IF __lnDoStackCount<=0 AND __lnIfStackCount<=0
				__lnForStackCount=__lnForStackCount+1
				IF __lnForStackCount<=1
					__lnAtPos=ATC(" TO ",__lcExpr)
					IF __lnAtPos=0
						__lcForExpr=""
						__lnForMax=0
						__lnForStep=0
						LOOP
					ENDIF
					__lcForExpr=__lcExpr
					__lcForExpr=ALLTRIM(LEFT(__lcExpr,__lnAtPos-1))
					__lcExpr=ALLTRIM(SUBSTR(__lcExpr,__lnAtPos+4))
					__lnAtPos=ATC("=",__lcForExpr)
					IF __lnAtPos=0
						LOOP
					ENDIF
					&__lcForExpr
					__lcForExpr=ALLTRIM(LEFT(__lcForExpr,__lnAtPos-1))
					__lnAtPos=ATC(" STEP ",__lcExpr)
					IF __lnAtPos=0
						__lnForMax=EVALUATE(__lcExpr)
						__lnForStep=1
					ELSE
						__lnForMax=EVALUATE(LEFT(__lcExpr,__lnAtPos-1))
						__lnForStep=EVALUATE(SUBSTR(__lcExpr,__lnAtPos+6))
					ENDIF
					__lnForLine=__lnLine
					LOOP
				ENDIF
			ENDIF
		CASE __lcCommand=="IF"
			IF __lnDoStackCount<=0 AND __lnForStackCount<=0
				__lnIfStackCount=__lnIfStackCount+1
				IF __lnIfStackCount<=1
					__lcIfExpr=__lcExpr
					__llIfExpr=EVALUATE(__lcIfExpr)
					__lnIfLine=__lnLine
					LOOP
				ENDIF
			ENDIF
		CASE __lcCommand=="ELSE"
			IF __lnIfStackCount=1 AND __lnDoStackCount<=0 AND __lnForStackCount<=0
				__llIfExpr=(NOT __llIfExpr)
				LOOP
			ENDIF
		CASE __lcCommand=="ENDD"
			IF __lnIfStackCount<=0 AND __lnForStackCount<=0
				__lnDoStackCount=__lnDoStackCount-1
				IF __lnDoStackCount<=0
					DO WHILE NOT EMPTY(__lcDoExpr) AND EVALUATE(__lcDoExpr)
						__lvResult=RunCode(@__laDoLines,.F.,__tvIgnoreErrors)
						IF ISNULL(__laDoLines[1])
							IF __llArrayCode
								__tcCode[1]=.NULL.
							ENDIF
							IF EMPTY(__lcLastOnError)
								ON ERROR
							ELSE
								ON ERROR &__lcLastOnError
							ENDIF
							RETURN __lvResult
						ENDIF
						IF NOT __lvResult
							EXIT
						ENDIF
					ENDDO
					__lcDoExpr=""
					__llDoExpr=.F.
					__lnDoLine=0
					DIMENSION __laDoLines[1]
					__laDoLines=.F.
					__lnDoLineTotal=0
					LOOP
				ENDIF
			ENDIF
		CASE __lcCommand=="ENDF"
			IF __lnIfStackCount<=0 AND __lnDoStackCount<=0
				__lnForStackCount=__lnForStackCount-1
				IF __lnForStackCount<=0
					IF __lnForStep>0
						DO WHILE EVALUATE(__lcForExpr)<=__lnForMax
							__lvResult=RunCode(@__laForLines,.F.,__tvIgnoreErrors)
							IF ISNULL(__laForLines[1])
								IF __llArrayCode
									__tcCode[1]=.NULL.
								ENDIF
								IF EMPTY(__lcLastOnError)
									ON ERROR
								ELSE
									ON ERROR &__lcLastOnError
								ENDIF
								RETURN __lvResult
							ENDIF
							IF NOT __lvResult
								EXIT
							ENDIF
							__lcExpr=__lcForExpr+"="+__lcForExpr+"+"+TRANSFORM(__lnForStep)
							&__lcExpr
						ENDDO
					ELSE
						DO WHILE EVALUATE(__lcForExpr)>=__lnForMax
							__lvResult=RunCode(@__laForLines,.F.,__tvIgnoreErrors)
							IF ISNULL(__laForLines[1])
								IF __llArrayCode
									__tcCode[1]=.NULL.
								ENDIF
								IF EMPTY(__lcLastOnError)
									ON ERROR
								ELSE
									ON ERROR &__lcLastOnError
								ENDIF
								RETURN __lvResult
							ENDIF
							IF NOT __lvResult
								EXIT
							ENDIF
							__lcExpr=__lcForExpr+"="+__lcForExpr+"+"+TRANSFORM(__lnForStep)
							&__lcExpr
						ENDDO
					ENDIF
					__lcForExpr=""
					__lnForCount=0
					__lnForMax=0
					__lnForStep=0
					__lnForLine=0
					DIMENSION __laForLines[1]
					__laForLines=.F.
					__lnForLineTotal=0
					LOOP
				ENDIF
			ENDIF
		CASE __lcCommand=="ENDI"
			IF __lnDoStackCount<=0 AND __lnForStackCount<=0
				__lnIfStackCount=__lnIfStackCount-1
				IF __lnIfStackCount<=0
					__lvResult=RunCode(@__laIfLines,.F.,__tvIgnoreErrors)
					IF ISNULL(__laIfLines[1])
						IF __llArrayCode
							__tcCode[1]=.NULL.
						ENDIF
						IF EMPTY(__lcLastOnError)
							ON ERROR
						ELSE
							ON ERROR &__lcLastOnError
						ENDIF
						RETURN __lvResult
					ENDIF
					__lcIfExpr=""
					__llIfExpr=.F.
					__lnIfLine=0
					DIMENSION __laIfLines[1]
					__laIfLines=.F.
					__lnIfLineTotal=0
					LOOP
				ENDIF
			ENDIF
	ENDCASE
	IF __lnDoStackCount>0
		__lnDoLineTotal=__lnDoLineTotal+1
		DIMENSION __laDoLines[__lnDoLineTotal]
		__laDoLines[__lnDoLineTotal]=__lcLine
		LOOP
	ENDIF
	IF __lnForStackCount>0 AND __lnDoStackCount<=0
		__lnForLineTotal=__lnForLineTotal+1
		DIMENSION __laForLines[__lnForLineTotal]
		__laForLines[__lnForLineTotal]=__lcLine
		LOOP
	ENDIF
	IF __lnIfStackCount>0
		IF NOT __llIfExpr
			LOOP
		ENDIF
		__lnIfLineTotal=__lnIfLineTotal+1
		DIMENSION __laIfLines[__lnIfLineTotal]
		__laIfLines[__lnIfLineTotal]=__lcLine
		LOOP
	ENDIF
	DO CASE
		CASE __lcCommand=="RETU"
			IF __llArrayCode
				__tcCode[1]=.NULL.
			ENDIF
			IF NOT EMPTY(__lcExpr)
				__lvResult=EVALUATE(__lcExpr)
			ENDIF
			EXIT
		CASE __lcCommand=="EXIT"
			IF __llArrayCode
				__lvResult=.F.
			ENDIF
			EXIT
		CASE __lcCommand=="TEXT"
			__llTextMode=.T.
			LOOP
		CASE __lcCommand=="ENDT"
			__llTextMode=.F.
			LOOP
	ENDCASE
	&__lcLine
ENDDO
IF EMPTY(__lcLastOnError)
	ON ERROR
ELSE
	ON ERROR &__lcLastOnError
ENDIF
RETURN __lvResult



FUNCTION __RunCodeError(tnError,tnLine,tcMethod,tcLine,tcMessage)
LOCAL lcMessage

IF _vfp.StartMode>0
	RETURN
ENDIF
lcMessage="RunCode Runtime Error"+CR_LF+ ;
		REPLICATE("-",40)+CR_LF+ ;
		"Error:      "+TRANSFORM(tnError)+CR_LF+ ;
		TRANSFORM(tcMessage)+CR_LF+ ;
		REPLICATE("-",40)+CR_LF+ ;
		"Method:  "+TRANSFORM(tcMethod)+CR_LF+ ;
		"Line        "+TRANSFORM(tnLine)+CR_LF+ ;
		REPLICATE("-",40)+CR_LF+ ;
		TRANSFORM(tcLine)

IF INLIST(_VFP.StartMode,0,4)
	WAIT CLEAR
	WAIT WINDOW LEFT(lcMessage,254) NOWAIT
ENDIF
ENDFUNC


*-- end RunCode.PRG
