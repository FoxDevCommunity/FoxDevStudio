*- PIVTABLE.H
*- #DEFINEs for PIVTABLE.VCX

#DEFINE L_DEBUG .F.

*- localize these
#DEFINE ALERTTITLE_LOC			"Microsoft Visual FoxPro Wizards"

#DEFINE		C_EXCLDBC1_LOC		"Excel cannot create a pivot table from the selected database '"
#DEFINE		C_EXCLDBC2_LOC		"' unless it is opened shared. Would you like the PivotTable Wizard to reopen it shared for you?"
#DEFINE 	C_STARTXL_LOC		"Starting Microsoft Excel..."
#DEFINE 	C_LARGEPIVOT_LOC	"You have selected a potentially large pivot table result set. "+;
								"Do you want to continue?"
#DEFINE 	C_NORECORDS_LOC		"The table selected contains no records. An Excel pivot table will not be created."
#DEFINE		C_LONGFNAME_LOC		"You have an old FoxPro ODBC Data Source installed which does not support FoxPro tables with long or illegal DOS path and file names. "+;
								"We suggest that you install the VFP ODBC Driver to prevent this message in the future. "+;
								"If you choose to continue, you must either rename or copy the selected table to one with a standard DOS file name. Would you like to copy the file now?"
#DEFINE 	C_COPYPROMPT_LOC	"Copy table to:"
#DEFINE		C_WAITMESS_LOC		"Please be patient while the pivot table is being generated..."

#DEFINE 	E_NOFORM_LOC		"No form file specified. Unable to continue."
#DEFINE		E_BADMSQUERY_LOC	"Microsoft Query has not been installed properly and is needed in order to create an Excel pivot table. Note: MS Query is an optional component that is not installed by default with newer versions of Office/Excel."
#DEFINE		E_FAILXL_LOC		"Failed to get Excel OLE Object."
#DEFINE 	E_NOREG_LOC			"Microsoft Excel is not properly registered in the Windows registration table for use with OLE."
#DEFINE		E_OLDXLVER_LOC		"The automation tool does not support your version of Microsoft Excel. You must have version 5.0 or later installed."
#DEFINE 	E_OLEERROR_LOC		"An OLE automation error occurred. Your Excel pivot table may not be complete."

*- error checks for the automation tool
#DEFINE		E_NEEDALIAS_LOC		"The Pivot Table tool requires the cAlias field to be filled."
#DEFINE		E_NEEDTABLE_LOC		"The Pivot Table tool requires the cDBCTable field to be filled."
#DEFINE		E_NEEDDBC_LOC		"The Pivot Table tool requires the cDBCName field to be filled."
#DEFINE		E_NEEDROW_LOC		"The Pivot Table tool requires the cPivFldRow field to be filled."
#DEFINE		E_NEEDCOL_LOC		"The Pivot Table tool requires the cPivFldCol field to be filled."
#DEFINE		E_NEEDDATA_LOC		"The Pivot Table tool requires the cPivFldData field to be filled."
#DEFINE		E_NEEDFIELDS_LOC	"The Pivot Table tool requires the aAutoFields array to be filled."
#DEFINE		E_NEEDFLDLIST_LOC	"The Pivot Table tool requires the aFldList array to be filled."

*- localize?
#DEFINE		OLE_XLAPP			"excel.application"

*- don't localize these		
#DEFINE		C_MSQRY32			"MSQRY32.EXE"
#DEFINE		C_PATH				"Path"
#DEFINE 	QUERY_ROOT			"Software\Microsoft\Shared Tools\MSQuery"

#DEFINE 	XL_CLASS			"Excel.Sheet"
#DEFINE 	XLCOUNT				-4113						&& Excel count function constant

#DEFINE MAX_RECORDS		500
#DEFINE MAX_PIVROWS		300
#DEFINE MAX_PIVCOLS		100
#DEFINE MAX_PIVPAGES	100
#DEFINE MAX_PIVCELLS	10000

#DEFINE		OS_W32S				1
#DEFINE		OS_NT				2
#DEFINE		OS_WIN95			3

#DEFINE 	HKEY_CLASSES_ROOT   -2147483648  && BITSET(0,31)
#DEFINE 	HKEY_CURRENT_USER   -2147483647  && BITSET(0,31)+1
#DEFINE 	HKEY_LOCAL_MACHINE  -2147483646  && (( HKEY ) 0x80000002 )

#DEFINE 	ERROR_SUCCESS		0
