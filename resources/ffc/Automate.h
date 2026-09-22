*- Automate.h
*-
*- #DEFINEs for the AUTOMATE.VCX class

#DEFINE L_DEBUG				.F.

#DEFINE E_UNSUPPORTEDOS_LOC	"This class cannot be used on the current operating system."
#DEFINE	E_ODBC1_LOC			"Could not check ODBC.INI file. Check to see if ODBC is properly installed."
#DEFINE	E_ODBC2_LOC			"Could not find FoxPro Files ODBC driver. Check to see if it is installed."
#DEFINE	E_ODBC3_LOC			"Invalid FoxPro Files entry in ODBC.INI file."
#DEFINE	E_ODBCDLL_LOC		"Could not check for proper ODBC installed files."
#DEFINE E_NOREGISTRY_LOC	"Unable to load REGISTRY procedure file. Make sure it is in the current path."
#DEFINE C_FILEUSE2_LOC		"File is in use. Please select another."
#DEFINE E_NODATA_LOC		"No data source was specified."
#DEFINE E_NOFIELDS_LOC		"No data fields were specified."
#DEFINE E_NOALIAS_LOC		"Specified alias could not be found."
#DEFINE C_ODBCOLDVER_LOC 	"You have an old FoxPro ODBC driver installed."


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

*- ODBC Driver info
#DEFINE ODBC_DRVR		"Driver"
#DEFINE ODBC_DRVR32		"Driver32"

#DEFINE		ODBC_FOX_DSN		"FoxPro Files"
#DEFINE		ODBC_FOX_FIL		"FoxPro 2.6"
#DEFINE 	FOXODBC_25			"FoxPro Files (*.dbf)"
#DEFINE 	FOXODBC_26			"Microsoft FoxPro Driver (*.dbf)"
#DEFINE 	FOXODBC_26FIX		"Microsoft FoxPro 2.6 Driver (*.dbf)"
#DEFINE 	FOXODBC_30			"Visual FoxPro"
#DEFINE 	FOXODBC_30a			"Visual FoxPro Database"
#DEFINE 	FOXODBC_30b			"Visual FoxPro Tables"
#DEFINE		C_DRIVEID			"DriverID"
#DEFINE		FOX_DRIVEID			24
#DEFINE 	ODBC_DATA_KEY		"Software\ODBC\ODBC.INI\"	&& ODBC Registry key
#DEFINE		C_FIL				"FIL"
#DEFINE		C_FOX2				"FoxPro 2.0"
#DEFINE 	HKEY_CLASSES_ROOT   -2147483648  && BITSET(0,31)
#DEFINE 	HKEY_CURRENT_USER   -2147483647  && BITSET(0,31)+1
#DEFINE 	HKEY_LOCAL_MACHINE  -2147483646  && (( HKEY ) 0x80000002 )

#DEFINE		FOXODBC_ANY			"FoxPro"

#DEFINE	ODBC_FILE 			"ODBC.INI"
#DEFINE	ODBC_FILE_68K 		"ODBC Preferences"
#DEFINE	ODBC_FILE_PPC 		"ODBC Preferences PPC"
#DEFINE	ODBC_SOURCE			"ODBC Data Sources"			&& Localization ????
#DEFINE	ODBC_32SOURCE		"ODBC 32 bit Data Sources"	&& Localization ????

*- registry codes
#DEFINE 	ERROR_SUCCESS		0
#DEFINE		ERROR_NOINIFILE		-108	&& no DLL file used to check ODBC
#DEFINE		ERROR_NOINIENTRY	-109	&& no entry found in INI file (section)
#DEFINE		ERROR_FAILINI		-110	&& no entry found in INI file

*- New #DEFINES for OLEDB support
#DEFINE		OLEDBPROVIDER_KEY		"VFPOLEDB\"
#DEFINE 	E_UNSUPPDBCEVENT_LOC	"DBC Events require the Visual FoxPro OLE DB Provider and " + ;
									"are not supported with the Microsoft Visual FoxPro ODBC driver. "
									
#DEFINE		E_NODBCEVENT_LOC		"DBC Events are turned on. Microsoft Office 2000 and earlier products " + ;
									"require ODBC, which will not work with DBC Events."
