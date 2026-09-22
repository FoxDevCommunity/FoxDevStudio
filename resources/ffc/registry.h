*- Registry.h
*- Copyright (c) 1997 Microsoft Corporation

* Operating System codes
#DEFINE	OS_W32S				1
#DEFINE	OS_NT				2
#DEFINE	OS_WIN95			3
#DEFINE	OS_MAC				4
#DEFINE	OS_DOS				5
#DEFINE	OS_UNIX				6

* DLL Paths for various operating systems
#DEFINE DLLPATH_NT			"\SYSTEM32\"
#DEFINE DLLPATH_WIN95		"\SYSTEM\"

* DLL files used to read INI files
#DEFINE	DLL_KERNEL_NT		"KERNEL32.DLL"
#DEFINE	DLL_KERNEL_WIN95	"KERNEL32.DLL"

* DLL files used to read registry
#DEFINE	DLL_ADVAPI_NT		"ADVAPI32.DLL"
#DEFINE	DLL_ADVAPI_WIN95	"ADVAPI32.DLL"

* DLL files used to read ODBC info
#DEFINE DLL_ODBC_NT			"ODBC32.DLL"
#DEFINE DLL_ODBC_WIN95		"ODBC32.DLL"

* Registry roots
#DEFINE HKEY_CLASSES_ROOT           -2147483648  && BITSET(0,31)
#DEFINE HKEY_CURRENT_USER           -2147483647  && BITSET(0,31)+1
#DEFINE HKEY_LOCAL_MACHINE          -2147483646  && BITSET(0,31)+2
#DEFINE HKEY_USERS                  -2147483645  && BITSET(0,31)+3

* Misc
#DEFINE APP_PATH_KEY		"\Shell\Open\Command"
#DEFINE OLE_PATH_KEY		"\Protocol\StdFileEditing\Server"
#DEFINE VFP_OPTIONS_KEY1	"Software\Microsoft\VisualFoxPro\"
#DEFINE VFP_OPTIONS_KEY2	"\Options"
#DEFINE CURVER_KEY			"\CurVer"
#DEFINE ODBC_DATA_KEY		"Software\ODBC\ODBC.INI\"
#DEFINE ODBC_DRVRS_KEY		"Software\ODBC\ODBCINST.INI\"
#DEFINE SQL_FETCH_NEXT		1
#DEFINE SQL_NO_DATA			100

* Error Codes
#DEFINE ERROR_SUCCESS		0	&& OK
#DEFINE ERROR_EOF 			259 && no more entries in key

* Note these next error codes are specific to this Class, not DLL
#DEFINE ERROR_NOAPIFILE		-101	&& DLL file to check registry not found
#DEFINE ERROR_KEYNOREG		-102	&& key not registered
#DEFINE ERROR_BADPARM		-103	&& bad parameter passed
#DEFINE ERROR_NOENTRY		-104	&& entry not found
#DEFINE	ERROR_BADKEY		-105	&& bad key passed
#DEFINE	ERROR_NONSTR_DATA	-106	&& data type for value is not a data string
#DEFINE ERROR_BADPLAT		-107	&& platform not supported
#DEFINE ERROR_NOINIFILE		-108	&& DLL file to check INI not found
#DEFINE ERROR_NOINIENTRY	-109	&& No entry in INI file
#DEFINE ERROR_FAILINI		-110	&& failed to get INI entry
#DEFINE ERROR_NOPLAT		-111	&& call not supported on this platform
#DEFINE ERROR_NOODBCFILE	-112	&& DLL file to check ODBC not found
#DEFINE ERROR_ODBCFAIL		-113	&& failed to get ODBC environment

* Data types for keys
#DEFINE REG_SZ 				1	&& Data string
#DEFINE REG_EXPAND_SZ 		2	&& Unicode string
#DEFINE REG_BINARY 			3	&& Binary data in any form.
#DEFINE REG_DWORD 			4	&& A 32-bit number.

* Data types labels
#DEFINE REG_BINARY_LOC		"*Binary*"			&& Binary data in any form.
#DEFINE REG_DWORD_LOC 		"*Dword*"			&& A 32-bit number.
#DEFINE REG_UNKNOWN_LOC		"*Unknown type*"	&& unknown type

* FoxPro ODBC drivers
#DEFINE FOXODBC_25			"FoxPro Files (*.dbf)"
#DEFINE FOXODBC_26			"Microsoft FoxPro Driver (*.dbf)"
#DEFINE FOXODBC_30			"Microsoft Visual FoxPro Driver"

