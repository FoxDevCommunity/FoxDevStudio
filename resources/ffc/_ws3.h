#DEFINE CRLF CHR(13)+CHR(10)

#DEFINE WSDLGEN_CLASS		"WSDLGEN.WSDLGenerator30"
#DEFINE SOAPCLIENT_CLASS	"MSSOAP.SoapClient30"
#DEFINE WSDLREADER_CLASS	"MSSOAP.WSDLReader30"
#DEFINE SOAPSERVER_CLASS	"MSSOAP.SoapServer30"
#DEFINE SOAPSERIAL_CLASS	"MSSOAP.SoapSerializer30"
#DEFINE SOAPREADER_CLASS	"MSSOAP.SoapReader30"
#DEFINE MSXML4_CLASS		"msxml2.domdocument.4.0"
#DEFINE SOAPCLIENT_INIT		MSSoapInit

#DEFINE HKEY_CLASSES_ROOT	-2147483648  && BITSET(0,31)
#DEFINE	ISAPIKEY	"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\MSSOAP\30\SOAPISAP\isapi"

#DEFINE DEFCLIENT_CLASS		"WSHandler"
#DEFINE DEFCLIENT_CLASSLIB	 [IIF(VERSION(2)=0,"",HOME()+"FFC\")+"_ws3client.vcx"]
#DEFINE DEFCLIENT_CLASSINIT	"SetupClient"

#DEFINE DEFPROXYGEN_CLASS	"WSProxy"
#DEFINE WSGEN_DEFAULTVAR	"loWS"
#DEFINE WSPROXYVAR			"loWSHandler"

#DEFINE	FOXWSDBF			"FOXWS3.DBF"
#DEFINE	FOXWS_OFFLINE_DBF	"FOXWS_OFFLINE.DBF"
#DEFINE	WSMARKER			"*__VFPWSDef__: "

#DEFINE	WSNONE_LOC			"(none)"
#DEFINE BADTYPELIB_LOC	"File does not appear to contain a valid typelib. If the DLL is a VFP COM server created on a Windows 9x platform, you should specify the TLB file instead."
#DEFINE BADPARM_LOC			"Invalid parameter passed."
#DEFINE NOINT_LOC			"Interface not found."
#DEFINE INVALIDPARM_LOC			"Invalid parameter passed."
#DEFINE NOWSDLFILE_LOC			"The WSDL or WSML file could not be found."

#DEFINE ASP_NOSOAPSVR_LOC		"Cannot create SoapServer object. "
#DEFINE ASP_SOAPINITFAIL_LOC	"SoapServer.Init failed. " 
#DEFINE ASP_SOAPINVOKEFAIL_LOC	"SoapServer.Invoke failed. " 
#DEFINE ASP_SOAPERROR_LOC		" SOAP ERROR: "
#DEFINE ASP_NOSEROBJ_LOC		"Could not create SoapSerializer object. "
#DEFINE ASP_INTERNALERR_LOC		"500 Internal Server Error"
#DEFINE ASP_SERINITFAIL_LOC		"SoapSerializer.Init failed. "
#DEFINE ASP_SERFAIL_LOC			"SoapSerializer failed. "
#DEFINE ASP_FAILREQUEST_LOC		"The request could not be processed due to a problem in the server. Please contact the system admistrator."
#DEFINE SELECTVIRDIR_LOC		"Select virtual directory location:"
#DEFINE CREATEVDERR_LOC			"Error creating new virtual directory. Check to see that IIS is working properly or name does not already exist."
#DEFINE VD_BADPARM_LOC			[Directory, name or path is missing or bad. Make sure name starts with "http://" and ends with "/".]
#DEFINE VDSUCCESS_LOC			"A default web service location was successfully set..."

#DEFINE HASVARIANT_LOC		"The selected class contains VARIANT return or parameter types that are not supported by SOAP."
#DEFINE APPNAME_SUFFIX		"Server"
#DEFINE WS_LOC				"web service"

#DEFINE ALLCLASS_LOC		"(all classes)"
#DEFINE NODLL_LOC	"You must first select a valid COM server file."
#DEFINE NOWSDL_LOC	"You must enter a WSDL file name for generation option(s) selected."
#DEFINE NOASP_LOC	"You must enter an ASP file name for generation option(s) selected."
#DEFINE NOURL_LOC	"You must enter an ASP listener URL for generation option(s) selected."
#DEFINE NOCLASS_LOC	"You must select a valid class name."
#DEFINE NOAPPNAME_LOC	"Please specify an application name for ASP listener."
#DEFINE NOISENSE_LOC	"Please specify a name for IntelliSense script."

#DEFINE GENFILE_LOC		"Generated..."
#DEFINE NOGENFILE_LOC	"Error generating "
#DEFINE WSDLFILES_LOC	"WSDL files."
#DEFINE ASPFILES_LOC	"ASP files."
#DEFINE INTELLISENSE_LOC	"IntelliSense scripts. Check for a valid WSDL URL location or invalid WSDL content. The WSDL file may have also been in use -- try regenerating the XML Web Service files again."
#DEFINE FINISH_LOC		"Finished generating files successfully."
#DEFINE FINISH2_LOC		"Finished generating IntelliSense scripts successfully."

#DEFINE FILENOTOPEN_LOC	"The XML Web Service data file could not be updated. Make sure it is not in use."
#DEFINE FIRSTTIME_LOC	"Before you can start publishing XML Web services, it is recommended that you set a local URL (virtual directory) to store generated XML Web service files. "
#DEFINE FIRSTTIME2_LOC	"This is a default location used to simplify publishing your XML Web Services. You will only need to set this once and can easily change it later."

#DEFINE MBCONFIGTITLE_LOC	"Visual FoxPro XML Web Services Configuration"
#DEFINE LBL1CAPTION1_LOC	"Select XML Web Service Location"
#DEFINE LBL1CAPTION2_LOC	"Select Default XML Web Service Location"
#DEFINE SAMPLEWSDL	"http://www.myserver.com/soap/myservice.wsdl"
#DEFINE SAMPLEISENSE_LOC	"Test Web Service"
#DEFINE NOSOAP_LOC	"The SOAP Toolkit (version 3.0 or higher) must first be installed to use web services."

#DEFINE NOIIS_LOC	"It does not appear that Internet Information Services (IIS) is installed on this computer. IIS requires Windows 2000 or higher. You will be able to create XML Web Service publishable documents, but not directly host them."

#DEFINE PHOOKCLASS "WSPHOOK"
#DEFINE PHBUILD_RETRIES	2
#DEFINE PHERROR_NOACTIVE_LOC	"Build failed. Make sure project or output file is not in use."
#DEFINE PHERROR_MISCERROR_LOC	"An error occurred trying to build the project. Make sure output file is not in use. Check the project err file for additional details. Error: "
#DEFINE PHBUILDOK_LOC 			"XML Web Service component was successfully rebuilt and supporting files regenerated."

#DEFINE MB_ERRISENSE_LOC	"Error generating XML Web Service IntelliSense scripts."
#DEFINE MB_GENWSDL_LOC	"Error generating WSDL files."

#DEFINE FOXWSINUSE_LOC		"Make sure your FOXWS3.DBF web services table is not opened."

#DEFINE MANYSERVICES_LOC 	"Note: the WSDL contains multiple services. The class name will be prefaced by the service it belongs to."
#DEFINE MBTITLE_WSGEN_LOC	"XML Web Services Publishing Results"

#DEFINE MB_WSFILESEXIST_LOC	"One or more web service files already exist. Would you like to overwrite them?"

#DEFINE	MB_ERRTITLE_LOC		"XML Web Services Publisher Error"

#DEFINE MB_NOVARIANT_LOC	"The selected class contains Currency or Variant return/parameter types. You should specify strong typing for these otherwise invalid WSDL may be generated."

#DEFINE MB_NONDEFSVR_LOC	"Your specified web service location does not appear to be a local path. Click on the Advanced... button to enter this information first."

#DEFINE HASPHOOK_LOC	"Yes"
#DEFINE HASNOPHOOK_LOC	"None"
#DEFINE PHOOKCONFLICT_LOC "Another project hook already exists."

#DEFINE LBL_NOIIS_LOC	"Since IIS is not installed on this machine, you will only have the option to select an existing web server location on another machine. If you select Cancel, you will be prompted later."

#DEFINE MB_EXISTWS_LOC	"The web service name specified is already registered to a different WSDL. Would you like to overwrite the existing entry?"

#DEFINE MB_NONLOCALSVR_LOC	"COM Servers on network locations are not supported. You must specify a local file."

#DEFINE MB_URLWITHSPACE_LOC	"The WSDL location contains a space which is not currently supported in URLs by the Visual FoxPro XML Web Service extensions."

#DEFINE MB_WSNOTFOUND_LOC	"The XML Web service you are trying to access is not registered. Would you like to register it now?"
#DEFINE MB_WSNOTFOUND1_LOC	[ If you are unable to register this service, you can prevent this prompt by renaming the object reference in its "__VFPWSDef__" reference above.]

#DEFINE ERR_LOCATION_LOC	Location:
#DEFINE ERR_NUMBER_LOC	Number:
#DEFINE ERR_METHOD_LOC	Method:
#DEFINE ERR_MESSAGE_LOC	Message:
#DEFINE ERR_LINE_LOC	Line:
#DEFINE ERR_MSG1_LOC	Press OK to ignore error and continue.
#DEFINE ERR_MSG2_LOC	Press Cancel to close.
#DEFINE	ERR_BADSOAPCLASS_LOC	"Could not instantiate SOAP client class."
#DEFINE	ERR_OTHERSOAPERR_LOC	"Other error."
#DEFINE	MB_SOAPERRTITLE_LOC		"XML Web Service Error"

#DEFINE MB_ERRNOWSDL_LOC	"Error retrieving XML Web service description."
#DEFINE MB_ERRNOMETHODS_LOC	"No methods are specified for the selected class. Pick methods you would like to include in your Web service from the Advanced dialog."
#DEFINE WAIT_QUERYWS_LOC	"Attempting to connect to XML Web service....."
#DEFINE WAIT_PUBWS_LOC		"Please be patient while publishing your XML Web service....."

#DEFINE	MB_UPDATEVDIR_LOC	"The Wizard has detected that your virtual directory is not setup to handle ISAPI listeners. Would you like to add a .WSDL application mapping to handle this?"
#DEFINE	MB_UPDATEVDIR2_LOC	"The Wizard has detected that your virtual directory has a .WSDL application mapping pointing to an ISAPI listener that is not the default SOAP 3.0 Toolkit one. Would you like to change this mapping to use the SOAP 3.0 Toolkit ISAPI listener?"

#DEFINE BTN_SELECT_LOC	"\<Select"
#DEFINE BTN_CREATE_LOC	"\<Create"

#DEFINE MB_REGFAIL1_LOC	"There was a problem accessing the "
#DEFINE MB_REGFAIL2_LOC	"Listener URI specified for the WSDL file."
#DEFINE MB_YES_LOC		"Yes"
#DEFINE MB_NO_LOC		"No"

#DEFINE PROPKEY_SUFFIX	"_KEY"

#DEFINE TRUE_LOC	"True"
#DEFINE FALSE_LOC	"False"
#DEFINE YES_LOC		"Yes"
#DEFINE NO_LOC		"No"

#DEFINE BADTYPE1_LOC	"The Web service could not be accessed."
#DEFINE BADTYPE2_LOC	"The Web service did not return a valid complex type (object)"

#DEFINE NOCOMPLEXPARM_LOC	"The builder cannot query complex types for parameters. You need to manually enter values."

#DEFINE SYNTAX_LOC		"Syntax: "
#DEFINE DESCRIPT_LOC	"Description: "
#DEFINE YOURPARMS_LOC	"Your Parameters: "
#DEFINE ISENSETYPE	"XML Web Service"


#DEFINE DEF_BINDPROP	"BindMe"
#DEFINE NOOBJECTS_LOC	"There are no objects to bind to. Please add an object to your form/container."

#DEFINE MB_NOCONTROL_LOC	"You must select a control to bind to your XML Web service."
#DEFINE MB_PROPNAME_LOC		"You must specify a property name to bind to your XML Web service."
#DEFINE MB_CLIENTNAME_LOC	"You must specify a client name."
#DEFINE MB_UNIQUEPEM_LOC	"You must specify a different binding property name which is unique."
#DEFINE MB_UNIQUEPEM2_LOC	"The Web service could not be bound to the following control because the binding property already exists: "

#DEFINE MB_BADIIS_LOC 		"Cannot run Wizard. You must be running IIS 5.0 or higher."

#DEFINE MB_DELETEWS_LOC		"Do you want to delete the selected Web service?"


#DEFINE MB_FILEINUSE_LOC	"Could not access Web service table FOXWS3. Make sure it is not in use."
#DEFINE MB_NODISPLAY_LOC	"Could not display the results of your Web service."
#DEFINE MB_NOWS_LOC  		"Call to the Web service failed:"
#DEFINE TEST_TMP_FILE  		"__testws.xml"
#DEFINE DATAVIEW_LOC		"Data View"
#DEFINE XMLVIEW_LOC		"XML View"
#DEFINE DOTNET_LOC		".Net Dataset Cursors:"
#DEFINE NOCURSOR_LOC	"Could not create a Fox cursor from XML"

#DEFINE ACTIONFAIL_LOC	"Failed to load XML Web service operation/client data."

#DEFINE INVOKE_ERROR_LOC	"There was an error calling the XML Web service."
#DEFINE BIND_ERROR_LOC		Error binding to XML Web Service.
#DEFINE ERROR_GENDATASET_LOC	"Error generating ADO .Net Dataset for binding."
#DEFINE ERROR_OPENCURSOR_LOC	"Error opening cursor from ADO .Net Dataset loaded in XMLAdapter."
#DEFINE ERROR_FAILOFFLINE_LOC	"Failed to open offline cache file."
#DEFINE ERROR_FAILOFFLINE2_LOC	"Failed to find or load offline data for Web service."

#DEFINE BADCALL_LOC	"This dialog can only be called from the parameters dialog."
#DEFINE CAPTION_LOC	"Zoom - "

#DEFINE UDDITABLE	"foxuddi.dbf"
#DEFINE OFFLINE_OBJREF_LOC	"(object)"

#DEFINE CHANGEWS_LOC	"Change XML Web service..."
#DEFINE ADDNEWWS_LOC	"Add a new XML Web service..."
#DEFINE OPENOPS_LOC	"You currently have Operations defined for this service. Changing the service will cause these Operations to be lost. Continue?"
#DEFINE DELETECLIENT_LOC "Do you want to delete selected client?"
#DEFINE DELETEOPERATION_LOC "Do you want to delete selected Operation?"
#DEFINE REFRESHWS_LOC	"The selected web service does not appear to be registered on this machine. Click on the Refresh button on the first page to register it."
#DEFINE SELECTWS_LOC	"Please select an operation first."

#DEFINE PARMSETTYPE1_LOC	"Input values now"
#DEFINE PARMSETTYPE2_LOC	"Programmatically set at run-time"
#DEFINE PARMSETTYPE3_LOC	"Prompt at run-time"

#DEFINE ENTERUDDINAME_LOC	"Enter name to seach for in UDDI database:"
#DEFINE UDDISEARCH_LOC		"UDDI Search"

#DEFINE MISSINGCLIENT_LOC	"The client control no longer appears to exist. It is likely that it or one of its parents was renamed. Please select a new control or cancel out of the builder and restore the original control."
#DEFINE REREGSITERWS_LOC	"The specified Web Service does not appear to be registered on this machine. You may not be able to access all the Operation and Client options. Click the Refresh link on the Services page to register this Web Service."

#DEFINE TMPWSCURSOR		"_vfptempws"

#DEFINE CONFIRM_ATTACHSCHEMA_LOC	"Do you want to attach the schema from the selected table to your cursoradapter?"
#DEFINE NOUDDIMATCH_LOC			"No UDDI matches found."

#DEFINE SAVEFORMFIRST_LOC	"You must first save form in order to properly read in control names."

#DEFINE REMOVECLIENTS_LOC  "You have changed the Web Service operation which may impact the associated clients. Would you like to remove these clients?"

#DEFINE NOFORMSET_LOC	"This visual XML Web Service is not supported on Formsets."

#DEFINE PROMPTFORMCLASSDE_LOC	"In order to view the DataEnvironement of a Form class, the builder first needs to instantiate the class. Would you like to do this?"

#DEFINE BINDCONTROLS_LOC	"You have selected to have the CursorAdapter populated by the Web service at startup. It is recommended that you set the Form BindControls property to .F. so that the Web Service control can automatically invoke it updates the CursorAdapter."
#DEFINE SEARCHUDDI_LOC	"Searching UDDI database for your request...."
#DEFINE XMLHASCURSOR_LOC	"The builder has detected that the Web service returns an XML string that can be converted to a Fox cursor. Would you like to view the resulting cursor schema?"