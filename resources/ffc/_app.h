* _app.h

***********************************************************
localization strings and constants for _app.vcx 
***********************************************************

* _datasession class
#DEFINE DATA_MESSAGEBOX_TITLE_LOC    "Data Message"
#DEFINE DATA_OK_TO_SAVE_LOC          "OK to save your edit?"
#DEFINE DATA_OK_TO_REVERT_LOC        "OK to cancel your changes?"
#DEFINE DATA_UPDATE_CONFLICT_LOC     "Records have been locked by another user. "+ ;
                                     CHR(13)+CHR(13) +;
                                     "You can't update these records until the other lock is cancelled."

#DEFINE DATA_HAS_BEEN_EDITED_LOC     "Other people may have edited the data since you started editing. "+CHR(13)+CHR(13) +;
                                     "OK to overwrite others' changes, "+CHR(13)+;
                                     "or cancel your edit for the records in this table?"
#DEFINE DATA_SAVE_BEFORE_CLOSE_LOC   "You have work in progress here."+CHR(13)+CHR(13)+; 
                                     "Do you want to save your changes before closing?"                                     
#DEFINE DATA_CANCEL_UNFINISHED_LOC   "You have work in progress here that cannot be saved yet."+CHR(13)+CHR(13)+; 
                                     "Do you want still want to close, cancelling your changes?"                                     
                                     

* _error class
* error logging
#DEFINE ERROR_MESSAGEBOX_TITLE_LOC   "Error Message" 
#DEFINE ERROR_IN_ERROR_METHOD_LOC    "Error in error handler"
#DEFINE ERROR_SERIOUS_CLASS_LOC      "Serious error of class"
#DEFINE ERROR_CANNOT_BE_LOGGED_LOC   "The application will exit, and cannot add information about this error to the error log."
#DEFINE ERROR_LOCK_LOC               "A file or record is unavailable"
#DEFINE ERROR_PRINT_LOC              "The printer or printer driver you require is not available"
#DEFINE ERROR_USER_FIX_LOC           "Please handle this problem, or wait and try again."      
#DEFINE ERROR_USER_NOTE_LOC          "Please note this error information."  

* error log display
#DEFINE ERROR_LOG_EMPTY_LOC          "The error log has no records."
#DEFINE ERROR_LOG_UNAVAILABLE_LOC    "The error log is not available."

* error continuation choices
#DEFINE ERROR_USER_CHOICES_LOC       "Continue Executing Program?"

#DEFINE ERROR_DEVEND_LOC             ERROR_USER_CHOICES_LOC+ ;
                                     CHR(13)+CHR(13)+;
                                     "Choose: "+CHR(13)+CHR(13)+ ;
                                     "YES to Continue the program"+CHR(13)+;
                                     "NO to Suspend "+CHR(13)+ ;
                                     "CANCEL to Exit program completely."

#DEFINE ERROR_USEREND_LOC            ERROR_USER_CHOICES_LOC+ ;
                                     CHR(13)+CHR(13)+;                                    
                                     "Choose: "+CHR(13)+CHR(13)+ ;
                                     "OK to Continue the program"+CHR(13)+;
                                     "CANCEL to Exit program completely."
               
#DEFINE ERROR_OCCURRED_LOC           "An error has occurred"                                   
#DEFINE ERROR_LOG_LOC                "Record details in error log files?"


***********************************************************
* Messagebox subset from FOXPRO.H
***********************************************************

#DEFINE MB_OK                   0       && OK button only
#DEFINE MB_OKCANCEL             1       && OK and Cancel buttons
#DEFINE MB_ABORTRETRYIGNORE     2       && Abort, Retry, and Ignore buttons
#DEFINE MB_YESNOCANCEL          3       && Yes, No, and Cancel buttons
#DEFINE MB_YESNO                4       && Yes and No buttons
#DEFINE MB_RETRYCANCEL          5       && Retry and Cancel buttons

#DEFINE MB_ICONSTOP             16      && Critical message
#DEFINE MB_ICONQUESTION         32      && Warning query
#DEFINE MB_ICONEXCLAMATION      48      && Warning message
#DEFINE MB_ICONINFORMATION      64      && Information message

#DEFINE MB_APPLMODAL            0       && Application modal message box
#DEFINE MB_DEFBUTTON1           0       && First button is default
#DEFINE MB_DEFBUTTON2           256     && Second button is default
#DEFINE MB_DEFBUTTON3           512     && Third button is default
#DEFINE MB_SYSTEMMODAL          4096    && System Modal

*-- MsgBox return values
#DEFINE IDOK            1       && OK button pressed
#DEFINE IDCANCEL        2       && Cancel button pressed
#DEFINE IDABORT         3       && Abort button pressed
#DEFINE IDRETRY         4       && Retry button pressed
#DEFINE IDIGNORE        5       && Ignore button pressed
#DEFINE IDYES           6       && Yes button pressed
#DEFINE IDNO            7       && No button pressed

***********************************************************
* Data-handling subset from FOXPRO.H
***********************************************************
*-- Cursor buffering modes
#DEFINE DB_BUFOFF               1
#DEFINE DB_BUFLOCKRECORD        2
#DEFINE DB_BUFOPTRECORD         3        
#DEFINE DB_BUFLOCKTABLE         4
#DEFINE DB_BUFOPTTABLE          5

*-- Update types for views/cursors
#DEFINE DB_UPDATE               1
#DEFINE DB_DELETEINSERT         2

*-- WHERE clause types for views/cursors
#DEFINE DB_KEY                  1
#DEFINE DB_KEYANDUPDATABLE      2
#DEFINE DB_KEYANDMODIFIED       3
#DEFINE DB_KEYANDTIMESTAMP      4

*-- Remote connection login prompt options
#DEFINE DB_PROMPTCOMPLETE       1
#DEFINE DB_PROMPTALWAYS         2
#DEFINE DB_PROMPTNEVER          3

*-- Remote transaction modes
#DEFINE DB_TRANSAUTO            1
#DEFINE DB_TRANSMANUAL          2

*-- Source Types for CursorGetProp()
#DEFINE DB_SRCLOCALVIEW         1
#DEFINE DB_SRCREMOTEVIEW        2
#DEFINE DB_SRCTABLE             3


***********************************************************
* System Toolbar subset from FOXPRO.H, Tastrade STRINGS.H
***********************************************************
*-- Toolbar Positions
#DEFINE TOOL_NOTDOCKED  -1
#DEFINE TOOL_TOP        0
#DEFINE TOOL_LEFT       1
#DEFINE TOOL_RIGHT      2
#DEFINE TOOL_BOTTOM     3

*-- Toolbar names
#DEFINE TB_FORMDESIGNER_LOC  "Form Designer"
#DEFINE TB_STANDARD_LOC      "Standard"
#DEFINE TB_LAYOUT_LOC        "Layout"
#DEFINE TB_QUERY_LOC		 "Query Designer"
#DEFINE TB_VIEWDESIGNER_LOC  "View Designer"
#DEFINE TB_COLORPALETTE_LOC  "Color Palette"
#DEFINE TB_FORMCONTROLS_LOC  "Form Controls"
#DEFINE TB_DATADESIGNER_LOC  "Database Designer"
#DEFINE TB_REPODESIGNER_LOC  "Report Designer"
#DEFINE TB_REPOCONTROLS_LOC  "Report Controls"
#DEFINE TB_PRINTPREVIEW_LOC  "Print Preview"

