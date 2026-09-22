
#define MESSAGE_LOC			"Microsoft Visual FoxPro"
#define ERRORTITLE_LOC		"Microsoft Visual FoxPro"
#define ERRORMESSAGE_LOC ;
	"Error #" + alltrim(str(m.nError)) + " in " + m.cMethod + ;
	" (" + alltrim(str(m.nLine)) + "): " + m.cMessage

#define MB_ICONEXCLAMATION		48
#define MB_ABORTRETRYIGNORE		2
#define MB_OK					0
#DEFINE MB_YESNO                4       && Yes and No buttons
#DEFINE IDYES           		6       && Yes button pressed

* These are the countries and regions to enable DBCS:  Japan, Korea, PRC, Taiwan
#DEFINE DBCS_LOC "81 82 86 88"

#DEFINE NUM_AFIELDS  18   			&& number of columns in AFIELDS array
#DEFINE DT_MEMO  	"M"
#DEFINE DT_GENERAL  "G"

#DEFINE TAGDELIM	 " *"

#DEFINE BMP_LOCAL		"dblview.bmp"
#DEFINE BMP_REMOTE		"dbrview.bmp"
#DEFINE BMP_TABLE		"dbtable.bmp"

#DEFINE C_FREETABLE_LOC		"Free Tables"
#DEFINE C_MAXFIELDS_LOC 	"The maximum number of fields to sort by is "
#DEFINE C_NOTAG_LOC 		"You cannot combine index tags and fields."
#DEFINE C_READONLY_LOC		"File is read-only and not allowed by this application. Please select another."
#DEFINE E_BADDBCTABLE_LOC	"The table selected does not have a valid backlink to its DBC. "+;
							"You can fix this with the VALIDATE DATABASE RECOVER command."
#DEFINE C_TPROMPT_LOC		"Select file to open:"
#DEFINE C_READ2_LOC			"File is used exclusively by another."
#DEFINE C_READ3_LOC			"File is in use. Select another."
#DEFINE C_READ4_LOC			"The DBF is part of a DBC. Select table from DBC container."
