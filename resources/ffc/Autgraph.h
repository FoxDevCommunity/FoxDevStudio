*- AUTGRAPH.H
*- #DEFINEs for AUTGRAPH.VCX

#DEFINE L_DEBUG .F.

*- localize these
#DEFINE ALERTTITLE_LOC			"Microsoft Visual FoxPro Wizards"

#DEFINE		OS_W32S				1
#DEFINE		OS_NT				2
#DEFINE		OS_WIN95			3

#DEFINE 	HKEY_CLASSES_ROOT   -2147483648  && BITSET(0,31)
#DEFINE 	HKEY_CURRENT_USER   -2147483647  && BITSET(0,31)+1
#DEFINE 	HKEY_LOCAL_MACHINE  -2147483646  && (( HKEY ) 0x80000002 )

#DEFINE 	ERROR_SUCCESS		0

#DEFINE		C_GRAPHDBF			"vfpgraph.dbf"
#DEFINE		C_GRAPHSCX			"vfpgraph.scx"

#DEFINE MSGRAPH_CLASS		"MSGraph.Chart"		&& version of MS Graph needed
#DEFINE MSGRAPH_8_CLASS		"MSGraph.Chart.8"	&& class of MS Graph needed
#DEFINE MSGRAPH_VERSION		5					&& version of MS Graph needed
#DEFINE MSGRAPH_APPNAME		"Microsoft Graph"	&& app name returned by application.name
#DEFINE MAX_MSGRAPH 		4000
#DEFINE MAX_DATAPOINTS		100
#DEFINE PIETYPE 			5
#DEFINE TAB 				CHR(9)
#DEFINE CRLF 				CHR(13)+CHR(10)
#DEFINE IS_NO				7
#DEFINE IS_YES				6
#DEFINE IS_CANCEL			2
#DEFINE OKCAN_DIALOG		33
#DEFINE YESNOCAN_DIALOG		35
#DEFINE YESNO_DIALOG		36
#DEFINE NO_BTN				256
#DEFINE I_DEFAULTGALLERY	21

#DEFINE C_BADMSGRAPH_LOC	"Microsoft Graph does not appear to be installed properly. The latest version of Graph is available from Microsoft Office."
#DEFINE C_NOMSGRAPH_LOC		"Could not locate MS Graph file. Please reinstall it from Microsoft Office."
#DEFINE C_MSGRAPHVER_LOC	"You must have MS Graph version 8.0 loaded. Please install correct version from Microsoft Office."
#DEFINE C_NOSOURCE_LOC		"No datasource selected. Graph automation tool terminated."
#DEFINE C_SAVEPROMPT1_LOC	"Save graph in table:"
#DEFINE C_SAVEPROMPT2_LOC	"Save graph in query:"
#DEFINE C_SAVEPROMPT3_LOC	"Save graph in form:"
#DEFINE C_FILEINUSE_LOC		"The table you selected is already in use."
#DEFINE C_READONLY_LOC		"The table you selected is read-only."
#DEFINE C_NODATAPOINTS_LOC	"No data points to graph."
#DEFINE C_MAXGRAPH_LOC		"You have over " + ALLTRIM(STR(MAX_MSGRAPH)) + " records in your table. "+;
							"This exceeds the maximum allowed by MS Graph."
#DEFINE C_TOOMANYPOINTS_LOC	"You have over "+ALLTRIM(STR(MAX_DATAPOINTS))+" records to graph. "+;
							"The graph may be crowded and hard to read. "+;
							"Do you want to prepare the graph anyway?"
#DEFINE C_BADCATEGORY_LOC	"The category field specified is not in the selected table."
#DEFINE C_NOTNUMERIC_LOC	"The data fields chosen are not all numeric. "+;
							"Do you want to continue plotting only those series which are numeric?"
#DEFINE C_BADDATAFIELD_LOC	"One of the data fields is not in the selected table."
#DEFINE C_NODATAFLDS_LOC	"No numeric data fields were found"
#DEFINE C_WAITDATA_LOC		"Adding data to graph..."
#DEFINE C_WAITFORMAT_LOC	"Formatting graph..."
#DEFINE C_HADERROR_LOC		"An error occurred in writing your graph to the selected table. "+;
							"Check to see if the table is already in use."
#DEFINE C_OLEERROR_LOC		"Could not proceed because an OLE Error Occurred."

#DEFINE C_BADFIELDS_LOC		"The source graph table does not have a valid General field."
#DEFINE C_APPENDREC_LOC		"You have selected a table which already exists. "+;
							"Choose Yes to append your graph to the existing table or No to create a new table."
#DEFINE C_NOAPPENDREC_LOC	"You have selected a table which does not have a General field for adding your graph. "+;
							"Would you like to create a new table?"

#DEFINE C_CLOSE_LOC			"Close"
#DEFINE C_FORMCAPTION_LOC	"VFP Graph"
#DEFINE C_PRVWCAPTION_LOC	"Graph Preview"

#IF 1
	*- chart types for Graph5
	#DEFINE I_AREA_GRAPH		1
	#DEFINE I_AREA3D_GRAPH		9
	#DEFINE I_BAR_GRAPH			2
	#DEFINE I_BAR3D_GRAPH		10
	#DEFINE I_COLUMN_GRAPH		3
	#DEFINE I_COLUMN3D_GRAPH	11
	#DEFINE I_PIE_GRAPH			7
	#DEFINE I_PIE3D_GRAPH		8
	#DEFINE I_LINE_GRAPH		1
	#DEFINE I_LINE3D_GRAPH		12
	#DEFINE I_HILO_GRAPH		8
	#DEFINE I_HILOCOLOR_GRAPH	7
#ELSE
	#DEFINE I_AREA_GRAPH		76
	#DEFINE I_AREA3D_GRAPH		78
	#DEFINE I_BAR_GRAPH			57
	#DEFINE I_BAR3D_GRAPH		60
	#DEFINE I_COLUMN_GRAPH		51
	#DEFINE I_COLUMN3D_GRAPH	54
	#DEFINE I_PIE_GRAPH			5
	#DEFINE I_PIE3D_GRAPH		-4102
	#DEFINE I_LINE_GRAPH		4
	#DEFINE I_LINE3D_GRAPH		-4101
	#DEFINE I_HILO_GRAPH		88
	#DEFINE I_HILOCOLOR_GRAPH	88
#ENDIF
