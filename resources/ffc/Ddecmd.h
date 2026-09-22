*- These should not be localized -- 
*- always provide English version along with local language

#DEFINE ENG_OLE_APPACTIVATE		AppActivate
#DEFINE ENG_OLE_APPRESTORE		AppRestore
#DEFINE ENG_OLE_FILENEW			FileNew
#DEFINE ENG_OLE_FILEOPEN		FileOpen
#DEFINE ENG_OLE_TOOLSMACRO		ToolsMacro
#DEFINE ENG_OLE_MMERGEOPENSRC	MailMergeOpenDataSource
#DEFINE ENG_OLE_MMERGEDOCTYPE	MailMergeMainDocumentType

#DEFINE ENG_APPACTIVATE		'[AppActivate "Microsoft Word", 1]'
#DEFINE ENG_APPMAXIMIZE		'[If AppMaximize() = 0 Then AppMaximize]'
#DEFINE ENG_APPRESTORE		'[If AppRestore() = 0 Then AppRestore]'
#DEFINE ENG_DDEIFCHECK		"[IF CountWindows()<>0 THEN IF FilePrintPreview()=-1 THEN FilePrintPreview 0]"
#DEFINE ENG_FILENEW			'[FileNew]'
#DEFINE ENG_FILEOPEN		'[FileOpen "'
#DEFINE ENG_OPENDATAFILE	'[FileOpenDataFile .Name = "'


*- These should be localized --
*- these are the Word Basic commands in the local language
#DEFINE X_OLE_APPACTIVATE_LOC	AppActivate
#DEFINE X_OLE_APPRESTORE_LOC	AppRestore
#DEFINE X_OLE_FILENEW_LOC		FileNew
#DEFINE X_OLE_FILEOPEN_LOC		FileOpen
#DEFINE X_OLE_TOOLSMACRO_LOC		ToolsMacro
#DEFINE	X_OLE_MAILMERGEHELPER_LOC	"MailMergeHelper"
#DEFINE X_OLE_MMERGEOPENSRC_LOC	MailMergeOpenDataSource
#DEFINE X_OLE_MMERGEDOCTYPE_LOC		MailMergeMainDocumentType

#DEFINE X_APPACTIVATE_LOC	'[AppActivate "Microsoft Word", 1]'
#DEFINE X_APPMAXIMIZE_LOC	'[If AppMaximize() = 0 Then AppMaximize]'
#DEFINE X_APPRESTORE_LOC	'[If AppRestore() = 0 Then AppRestore]'
#DEFINE X_DDEIFCHECK_LOC	"[IF CountWindows()<>0 THEN IF FilePrintPreview()=-1 THEN FilePrintPreview 0]"
#DEFINE X_FILENEW_LOC		'[FileNew]'
#DEFINE X_FILEOPEN_LOC		'[FileOpen "'
#DEFINE X_OPENDATAFILE_LOC	'[FileOpenDataFile .Name = "'