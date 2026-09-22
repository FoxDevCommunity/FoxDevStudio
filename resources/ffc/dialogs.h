*-- (c) Microsoft Corporation 1995

*-- Common include file
#INCLUDE "FOXPRO.H"

#DEFINE DEBUGMODE	.F.
#DEFINE INIFILE		"TASTRADE.INI"
#DEFINE CRLF		CHR(13) + CHR(10)
#DEFINE CR			CHR(13)
#DEFINE TAB			CHR(9)

#DEFINE CURRENCY	"$"
#DEFINE AERRORARRAY	7

*-- Constants used to read the system registry
#DEFINE HKEY_LOCAL_MACHINE			-2147483646  
#DEFINE KEY_SHARED_TOOLS_LOCATION	"Software\Microsoft\Shared Tools Location"
#DEFINE KEY_NTCURRENTVERSION		"Software\Microsoft\Windows NT\CurrentVersion"
#DEFINE KEY_WIN4CURRENTVERSION		"Software\Microsoft\Windows\CurrentVersion"
#DEFINE KEY_WIN4_MSINFO				"Software\Microsoft\Shared Tools\MSInfo"
#DEFINE KEY_QUERY_VALUE				1
#DEFINE ERROR_SUCCESS				0

#DEFINE DOLLAR_FORMAT1_LOC ": $"
#DEFINE DOLLAR_FORMAT2_LOC ""
#DEFINE DOLLAR_FORMAT3_LOC "$"

#DEFINE NOUSERS_LOC			"There are no users on file."
#DEFINE BADPASSWORD_LOC		"Password is invalid."
#DEFINE NOOPENTABLE_LOC	"Could not open password table."

#DEFINE COPYRIGHT_LOC		"Copyright 1996 Microsoft Corporation"
#DEFINE RIGHTSRSRVD_LOC		"All rights reserved"
#DEFINE ABOUT_LOC			"About "
#DEFINE VERSIONLABEL_LOC    "Version "

#DEFINE FILENOTFOUND1_LOC	[Error opening "]
#DEFINE FILENOTFOUND2_LOC	[".  File not found.]

#DEFINE M_REMOVE_KEYWORD_LOC		"Remove keyword"
#DEFINE M_KEYWORD_LOC				"Keyword"
#DEFINE M_KEYWORDS_LOC				"Keywords"
#DEFINE M_KEYWORDS_NO_SPACES_LOC	"Keywords may not contain spaces"
#DEFINE M_ALREADY_EXISTS_LOC		"already exists"
#DEFINE M_ALREADY_EXISTS_OVER_LOC	" already exists, overwrite it?"
#DEFINE M_ALREADY_IN_USE_LOC		"already in use"
#DEFINE M_UPDATING_KEYWORDS_LOC		"Updating Keywords"
#DEFINE M_NOKEYWORDTABLE_LOC		"Could not find keywords table."
#DEFINE M_NOOPENKEYWORDTABLE_LOC	"Could not open keywords table."
