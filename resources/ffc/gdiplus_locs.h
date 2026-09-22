*
*	GDI+ Localisation strings
*
#ifndef _GDIPLUS_LOCS_H_INCLUDED


* Error messages
#define _GDIPLUS_GDIPLUSNOTINIT_LOC			'GDI+ not initialized'
#define _GDIPLUS_NOGDIPOBJECT_LOC			'GDI+ object not created or associated'
#define _GDIPLUS_GDIPNOTOWNED_LOC			'GDI+ object not owned by VFP object'
#define _GDIPLUS_INTERNALBUFTOOSMALL_LOC	'Internal error: buffer too small'
#define _GDIPLUS_STRINGTOGUID_LOC			'StringToGUID error code ' + ltrim(str(m.lnResult))
#define _GDIPLUS_MALLOCFAIL_LOC				'Memory allocation failed'
#define _GDIPLUS_BADPROPERTYTAGTYPE_LOC		'Unknown or invalid property tag type'
#define _GDIPLUS_BADENCODERPARAMSTRING_LOC	'Invalid encoder parameter string'
#define _GDIPLUS_BADENCODERPARAMNAME_LOC	'Invalid encoder parameter name "'+m.lcName+'"'
#define _GDIPLUS_BADENCODERPARAMNAMETYPE_LOC	'Invalid data type for encoder parameter name'
#define _GDIPLUS_BADENCODERPARAMVALUE_LOC	'Invalid encoder parameter value'

* Error handler
#define _GDIPLUS_ERRNOLABEL_LOC				"Error:           "
#define _GDIPLUS_ERRPROCLABEL_LOC			"Method:       "
#define _GDIPLUS_ERRLINELABEL_LOC			"Line:            "



#endif && _GDIPLUS_LOCS_H_INCLUDED