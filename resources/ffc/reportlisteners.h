#INCLUDE FOXPRO_REPORTING.H

#INCLUDE REPORTLISTENERS_LOCS.H

* -- constants for VCX super classes 

* --  general constant for debugging
#DEFINE OUTPUTCLASS_DEBUGGING .F.

* -- UtilityReportListener class 

#DEFINE OUTPUTCLASS_READCONFIG_NEITHER           0
#DEFINE OUTPUTCLASS_READCONFIG_INIT              1
#DEFINE OUTPUTCLASS_READCONFIG_REPORT            2
#DEFINE OUTPUTCLASS_READCONFIG_BOTH              3

* -- XML Output Listener class-specific constants

#DEFINE OUTPUTXML_DATA_ONLY 0
#DEFINE OUTPUTXML_RDL_ONLY    1
#DEFINE OUTPUTXML_DATA_RDL    2

#DEFINE OUTPUTXML_BREAKS_INDATA          0
#DEFINE OUTPUTXML_BREAKS_NONE              1
#DEFINE OUTPUTXML_BREAKS_COLLECTION   2

#DEFINE OUTPUTXML_RAW                 0
#DEFINE OUTPUTXML_DOM                  1
#DEFINE OUTPUTXML_DOTNET            2

#DEFINE OUTPUTXML_CHARFIELD_LIMIT 254

#DEFINE OUTPUTXML_XSLTOBJECT_TYPESTRINGS "|document|element|"

* #DEFINE OUTPUTXML_REPEATSPANNEDITEM .T.
* #DEFINE OUTPUTXML_PERFORMLOCALECONVERSION .F.

#DEFINE OUTPUTXML_CONTINUATION (THIS.NoPageEject OR ;
                               (VARTYPE(THIS.CommandClauses) = "O" AND ;
                               THIS.CommandClauses.NoPageEject))

* supplied superclasses' tunable settings

#DEFINE OUTPUTCLASS_INTERNALDBF  "_ReportOutputConfig"
#DEFINE OUTPUTCLASS_EXTERNALDBF  "OutputConfig"

#DEFINE OUTPUTCLASS_OBJTYPE_CONFIG         1000

#DEFINE OUTPUTCLASS_STATUSCHAR_PCT_DONE         [|]
#DEFINE OUTPUTCLASS_STATUSCHAR_PCT_NOT_DONE     [.]     
#DEFINE OUTPUTCLASS_ONE_HUNDRED_PCT_MARK        50

#DEFINE OUTPUTCLASS_FILENAME_CHARS_DISALLOWED  [?*"<>|]

* -- XML Output Listener-specific user-tunable settings

#DEFINE OUTPUTXML                                  OUTPUTXML_RAW

#DEFINE OUTPUTXML_OBJTYPE_NODES       1100
#DEFINE OUTPUTXML_OBJTYPE_BANDOFFSET   500
#DEFINE OUTPUTXML_OBJCODE_DOC          100
#DEFINE OUTPUTXML_OBJCODE_DATA         200
#DEFINE OUTPUTXML_OBJCODE_RDL          300
#DEFINE OUTPUTXML_OBJCODE_PAGES        400
#DEFINE OUTPUTXML_OBJCODE_COLS         500
#DEFINE OUTPUTXML_OBJCODE_RUN          550
#DEFINE OUTPUTXML_OBJCODE_ATTRIBMEMBER 600

#DEFINE OUTPUTXML_GOOFTAG  "XXXX"
   

*&* The default Sedna MSXML-related #DEFINEs are the same as 
*&* in previous VF9 releases:

#DEFINE OUTPUTXML_DOMDOCUMENTOBJECT "Msxml2.FreeThreadedDOMDocument.4.0"
#DEFINE OUTPUTXML_DOMFREETHREADED_DOCUMENTOBJECT "Msxml2.FreeThreadedDOMDocument.4.0"
#DEFINE OUTPUTXML_XSLT_PROCESSOROBJECT "Msxml2.XSLTemplate.4.0"

*&* Be sure to verify the availability of
*&* specific-to-VFP MSXML versions in your 
*&* distributed applications, if you do not
*&* opt to change these #DEFINEs for Vista use.
*&* If you are not using any other XML-related
*&* features in your distributed application besides
*&* Reporting elements, such as XMLAdapter's Attach and 
*&* LoadXML methods, you may prefer the instructions below.

*&* For deployment on Vista only,
*&* without explicit addition of MSXML
*&* versions to support VFP,
*&* the following versions will be available, and
*&* they contain the required features:
 
*&*	 #DEFINE OUTPUTXML_DOMDOCUMENTOBJECT "Msxml2.FreeThreadedDOMDocument.6.0"
*&*	 #DEFINE OUTPUTXML_DOMFREETHREADED_DOCUMENTOBJECT "Msxml2.FreeThreadedDOMDocument.6.0"
*&*	 #DEFINE OUTPUTXML_XSLT_PROCESSOROBJECT "Msxml2.XSLTemplate.6.0"

*&* Version-independent programming against MSXML is usually
*&* not recommended.  It is not even supported, in later versions
*&* of MSXML. See
*&* http://msdn.microsoft.com/library/default.asp?url=/library/en-us/xmlsdk/html/8e50f590-3820-41eb-9a4e-82d58b90de8d.asp

*&* However, the version-independent #DEFINEs below may also be 
*&* substituted without error if you wish to support 
*&* both Windows XP and Vista 
*&* without the requirement of adding any specific-for-VFP 
*&* MSXML versions to Vista:

*&* #DEFINE OUTPUTXML_DOMDOCUMENTOBJECT "Msxml2.FreeThreadedDOMDocument"
*&* #DEFINE OUTPUTXML_DOMFREETHREADED_DOCUMENTOBJECT "Msxml2.FreeThreadedDOMDocument"
*&* #DEFINE OUTPUTXML_XSLT_PROCESSOROBJECT "Msxml2.XSLTemplate"

*&* Downgrade back to the stipulated versions above in any 
*&* side-by-side installation scenarios, 
*&* especially in earlier OS environments. 
*&* The default MSXML version available on the target 
*&* computer and invoked by the version-independent calls
*&* may be lower than our required versions even where these
*&* versions are actually present.  

#DEFINE OUTPUTFX_BASERENDER_AFTERRESTORE          0  
#DEFINE OUTPUTFX_BASERENDER_RENDER_BEFORE_RESTORE 1
#DEFINE OUTPUTFX_BASERENDER_NORENDER              2
#DEFINE OUTPUTFX_BASERENDER_RENDERXBASEONLY       3
*&* The following two values may not have any 
*&* practical use, because Xbase listeners may not
*&* have any practical way of making this distinction,
*&* so the previous value should be used for now.
*&* They are designated here for completeness:
#DEFINE OUTPUTFX_BASERENDER_RENDERXBASEONLY_AFTER      4
#DEFINE OUTPUTFX_BASERENDER_RENDERXBASEONLY_BEFORE     5


#DEFINE OUTPUTFX_DEFAULT_RENDER_BEHAVIOR          OUTPUTFX_BASERENDER_AFTERRESTORE

#DEFINE OUTPUTFX_RUNCOLLECTOR_RESET_NEVER      0
#DEFINE OUTPUTFX_RUNCOLLECTOR_RESET_ONREPORT   1
#DEFINE OUTPUTFX_RUNCOLLECTOR_RESET_ONCHAIN    2

#DEFINE OUTPUTFX_ADDCOLLECTION_NOACTION         0
#DEFINE OUTPUTFX_ADDCOLLECTION_FAILURE         -1
#DEFINE OUTPUTFX_ADDCOLLECTION_UNSUITABLE      -2
#DEFINE OUTPUTFX_ADDCOLLECTION_SUCCESS          1

#DEFINE OUTPUTHTML_DEFAULT_PAGEIMAGE_TYPE       LISTENER_DEVICE_TYPE_GIF
#DEFINE OUTPUTFILE_MAX_FILEPLACES               7