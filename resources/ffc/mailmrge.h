*- MAILMRGE.H
*- #DEFINEs for MAILMRGE.VCX

#DEFINE L_DEBUG .F.

*- localize these
#DEFINE C_STARTWORD60_LOC	"Starting Microsoft Word ..."
#DEFINE C_STARTWORD20_LOC	"Starting Microsoft Word 2.0..."
#DEFINE C_STARTWORD80_LOC	"Starting Microsoft Word ..."
#DEFINE C_OPENDATA80_LOC	"Opening data source..."
#DEFINE C_MMSAVEAS_LOC		"Save merge data as:"
#DEFINE C_DFLTNAME_LOC		"mrgdata.txt"
#DEFINE C_DFLTDBF_LOC		"mrgdata.dbf"

#DEFINE E_NOOPENWORD_LOC	"Failed to get Word OLE Object."
#DEFINE E_FILEERR_LOC		"Error writing merge data."
#DEFINE E_NOMSWLAUNCH_LOC	"Cannot start Microsoft Word."
#DEFINE E_WRONGWORD_LOC		"The wrong version of Microsoft Word is running. Mail merge canceled."
#DEFINE E_NOWORDERR_LOC		"Microsoft Word for Windows @1 can not be located on this computer."
#DEFINE E_NOWORDMACERR_LOC	"Microsoft Word for the Macintosh 6.0 is not installed on this computer."
#DEFINE E_MSWCOMMFAIL_LOC	"Cannot complete task. Check MS Word for open dialog or other problem..."
#DEFINE E_NODBC_LOC			"DBC Tables are not supported yet by ODBC drivers. You must use a FoxPro free table."
#DEFINE E_DDEERR_LOC		"The last DDE action was unsuccessful. The DDE error code is @1."

*- error messages for bad values in MailMerge class
#DEFINE E_BADWORDPROC_LOC	"Invalid value for word processor."
#DEFINE E_BADEXDOC_LOC		"No existing document was specified."
#DEFINE E_BADMRGDATA_LOC	"No destination for merge data was specified."
#DEFINE E_BADTEMPLATE_LOC	"Invalid value for Word template."
#DEFINE E_NODATA_LOC		"No data source was specified."
#DEFINE E_NOFIELDS_LOC		"No data fields were specified."

#DEFINE	E_ODBC1_LOC			"Could not check ODBC.INI file. Check to see if ODBC is properly installed."
#DEFINE	E_ODBC2_LOC			"Could not find FoxPro Files ODBC driver. Check to see if it is installed."
#DEFINE	E_ODBC3_LOC			"Invalid FoxPro Files entry in ODBC.INI file."
#DEFINE	E_ODBCDLL_LOC		"Could not check for proper ODBC installed files."

#DEFINE E_NOOPENTBL_LOC		"Cannot open data table for shared use. Mailmerge cancelled."
#DEFINE E_NOREPLACETBL_LOC	"Unable to replace file. Mailmerge cancelled."
#DEFINE E_NOAPPLESCRIPT_LOC	"AppleScript" + CHR(170) + " is not installed on this machine. Launch Word and manually complete the Mail Merge."

#DEFINE C_LOCATEDLL_LOC		"The wizard could not verify that you have ODBC installed. Do you want to continue with the assumption that it is?"
#DEFINE C_NOVERIFYMSW_LOC	"The wizard could not verify that you have Microsoft Word installed. Do you want to continue with the assumption that it is?"
#DEFINE C_COPYFOX2_LOC		"You do not have ODBC drivers for Visual FoxPro 3.0 tables. "+;
							"Would you like to copy the selected table to a FoxPro 2.5 "+;
							"type so that a merge data file can be generated from it?"
#DEFINE C_COPYFOX3_LOC		"You have selected a view as your data source. " + ;
							"You must copy the view so that a merge data file can be generated from it. Proceed?"
#DEFINE C_COPYPROMPT_LOC	"Copy table to:"	
#DEFINE C_ERROR_SQL_LOC		"The generated SQL statement is too long. Please select fewer fields."
#DEFINE C_LOCATE_LOC		"Locate "
#DEFINE	C_ODBCOLDVER_LOC	"The Version setting in your FoxPro Files ODBC Data Source is set for FoxPro 2.0 files. "+;
							"The MailMerge Wizard may not properly handle international characters or General fields. "+;
							"You can change this setting to FoxPro 2.6 using the ODBC Control Panel."
#DEFINE	C_EXCLDBC1_LOC		"Word cannot merge data from the selected database '"
#DEFINE	C_EXCLDBC2_LOC		"' unless it is opened shared. Would you like the MailMerge Wizard to reopen it shared for you?"
							
*- don't localize these

*- if these are changed, also change in MAILMRGE.H
*- supported word procs/merge types
#DEFINE		N_WORD60		1
#DEFINE		N_WORD20		3		&& not supported in 5.0
#DEFINE		N_COMMADELIM	2

*- new/old doc
#DEFINE		N_NEW_DOC		1
#DEFINE		N_EXISTING_DOC	2

*- template types
#DEFINE		N_FORMLETTER	1
#DEFINE		N_LABEL			2
#DEFINE		N_ENVELOPE		3
#DEFINE		N_CATALOG		4

*- used in startword
#DEFINE C_MSWORDMAC		"Microsoft Word"
#DEFINE C_MSWORDEXE		"winword.exe"
#DEFINE C_MSWORDWIN		"Winword"
#DEFINE C_MSWORDCREATOR	"MSWD"

#DEFINE C_WORD2				"2.0"
#DEFINE C_WORD6				"6.0"
#DEFINE C_WORD6_OR_LATER	"6.0"
#DEFINE C_WORD8				"8.0"
#DEFINE C_WORD10			"10"

*- MSW 2.0 values
#DEFINE C_WIN_SECT2			"Microsoft Word 2.0"

*- MSW 6.0 values
#DEFINE C_WIN_SECT6			"Microsoft Word"
#DEFINE C_WIN_MSW6INI		"Winword6.INI"

#DEFINE C_WIN_DIR			"programdir"

*- entry in Mac FoxPro settings file
#DEFINE C_FOXPROINI_MAC		"Visual Foxpro Settings"
#DEFINE C_WORD6ID			"Word6Loc"
#DEFINE C_MAILMRG_SECT		"Mailmerge"
#DEFINE C_WORDSETTINGS		"Word Settings (6)"

*- DDE Errors
#DEFINE N_BADCHANNELERR		 6
#DEFINE N_TIMEOUTERR		 9
#DEFINE N_NOCLIENTERR		13

*- AppleScript related errors
#DEFINE N_RUNSCRIPTFAIL		1921	&& VFP error numbers
#DEFINE N_SCRIPTERROR		1917

#DEFINE L_GETFILE			.T.
#DEFINE L_DONTGETFILE		.F.

#DEFINE K_HIGHTIMEOUT	20000
#DEFINE K_DFLTTIMEOUT	 2000

*- MSW 2.0 values
#DEFINE WIN_SECT2	"Microsoft Word 2.0"
#DEFINE WIN_DIR		"programdir"

*- MSW 6.0 values
#DEFINE WIN_SECT6	"Microsoft Word"
#DEFINE WIN_MSW6INI	"Winword6.INI"
#DEFINE WIN_6OBJ	"word.basic"

*- MSW 8.0 values
#DEFINE WIN_8OBJ	"word.application.8"

*- ODBC Driver info
#DEFINE K_TAB			CHR(9)
#DEFINE C_CRLF			CHR(13) + CHR(10)
#DEFINE C_CR			CHR(13)

#DEFINE K_TOOLONG		150		&& max length of a SQL statement for DDE
#DEFINE K_TRANSFER		1024 * 20

#DEFINE K_LOCALVIEW		1
#DEFINE K_REMOTEVIEW	2
#DEFINE K_TABLE			3

#DEFINE	DBFTYPE_30		48

#DEFINE EXT_DBF			"DBF"
#DEFINE EXT_TXT			"TXT"

******************************************************************************
* Used by GetOS and other methods
******************************************************************************
* Operating System codes
#DEFINE	OS_W32S				1
#DEFINE	OS_NT				2
#DEFINE	OS_WIN95			3
#DEFINE	OS_MAC				4
#DEFINE	OS_DOS				5
#DEFINE	OS_UNIX				6


#DEFINE	REG_MSWDOC_KEY		"Word.Document"
#DEFINE	REG_MSWAPP_KEY		"Word.Application"

#DEFINE	I_ENGLISH			1033			&& code for English

#DEFINE	FOXODBC_ANY			"FoxPro"

#DEFINE I_WDDIALOGMAILMERGEHELPER	680		&& Word code for Mailmerge Helper dialog

*- registry codes
#DEFINE	ERROR_SUCCESS		0
#DEFINE	ERROR_NOINIFILE		-108	&& no DLL file used to check ODBC
#DEFINE	ERROR_NOINIENTRY	-109	&& no entry found in INI file (section)
#DEFINE	ERROR_FAILINI		-110	&& no entry found in INI file

#DEFINE NOPARAMVIEW_LOC	"Parameterized views are not supported by Mail Merge Wizard."

*- eof  MAILMRGE.H