* _TABLE.H

***********************************
* constants for _Table* classes

#DEFINE DB_BUFLOCKRECORD        2
#DEFINE DB_BUFOPTRECORD         3    

#DEFINE FILTER_MAX_FILTER       255

#DEFINE MB_ICONEXCLAMATION   48
#DEFINE MB_YESNOCANCEL        3
#DEFINE IDYES                 6
#DEFINE IDNO                  7

***********************************
* localization for _Table* classes

* _TableFind* buttons and dialog class strings

#DEFINE FIND_LOOKFOR_LOC          "\<Look for:"
#DEFINE FIND_LOOKIN_LOC           "Look \<in:"
#DEFINE FIND_OPTIONS_LOC          "Options"
#DEFINE FIND_WRAPAROUND_LOC       "Wrap aroun\<d"
#DEFINE FIND_MATCHCASE_LOC        "\<Match case"
#DEFINE FIND_SKIPMEMOS_LOC        "\<Skip memos"
#DEFINE FIND_FIND_LOC             "\<Find"
#DEFINE FIND_FINDNEXT_LOC         "Find \<next"
#DEFINE FIND_CANCEL_LOC           "\<Cancel"
#DEFINE FIND_NOFIND_LOC           "Not found"
#DEFINE FIND_CAPTION_LOC          "Find"
#DEFINE FIND_FINDIN_LOC           "Find in"

***********************************
* _TableSetFilterDialog strings:

#DEFINE SETFILTER_CAPTION_LOC     "\<Set filter"
#DEFINE SETFILTER_APPLY_LOC       "\<Apply"
#DEFINE SETFILTER_MAXLENGTH_LOC   "Filter must be <255 characters."
#DEFINE SETFILTER_INVALID_LOC     "Filter must be logical in type."
#DEFINE SETFILTER_BUILDEXPR_LOC   "\<Build expression"
#DEFINE SETFILTER_SIMPLE_LOC      "Simple"
#DEFINE SETFILTER_COMPLEX_LOC     "Complex"
#DEFINE SETFILTER_CANCEL_LOC      "\<Cancel"
#DEFINE SETFILTER_EDIT_LOC        "\<Edit"
#DEFINE SETFILTER_NEED_LIB_LOC    "Need _TABLE.VCX library!"


**********************************
* _TableSimpleFilterDialog strings:

#DEFINE FILTER_CANCELLED_LOC             "Cancelled"
#DEFINE FILTER_SECONDS_LOC               "Seconds"
#DEFINE FILTER_TOO_LONG_LOC              "Expression too long"
#DEFINE FILTER_RECORDS_LOC               "Records"
#DEFINE FILTER_NUMERIC_REQUIRED_LOC      "Numeric field; a number is expected"
#DEFINE FILTER_NUMERIC_NO_QUOTES_LOC     "Numeric field; quotes are not allowed"
#DEFINE FILTER_MISSING_VALUE_LOC         "Missing value"
#DEFINE FILTER_CHECKING_OPEN_TABLES_LOC  "Checking open files.. "
#DEFINE FILTER_NO_SINGLE_QUOTES_LOC      "Can't have single quote in value"
#DEFINE FILTER_QUERY_LIST_FULL_LOC       "The query list is full." 

***********************************
* _Table class strings:

#DEFINE TABLE_MESSAGE_TITLE_ROW_CHANGED_LOC "Data in this row has been changed."

#DEFINE TABLE_MESSAGE_ROW_CHANGED_LOC       "Before moving to another record,"+CHR(13)+ ;
                                            "do you wish to: "+CHR(13)+CHR(13)+;
                                            "  SAVE changes  (Yes)"+CHR(13)+;
                                            "  REVERT changes (No) or"+CHR(13)+ ;
                                            "  REMAIN on the record (Cancel)?"
************************
* _dialog fonts
#DEFINE SYSTEM_LARGEFONTS            FONTMETRIC(1, 'MS Sans Serif', 8, '') # 13 OR ;
                                     FONTMETRIC(4, 'MS Sans Serif', 8, '') # 2 OR ;
                                     FONTMETRIC(6, 'MS Sans Serif', 8, '') # 5 OR ;
                                     FONTMETRIC(7, 'MS Sans Serif', 8, '') # 11

#DEFINE DIALOG_SMALLFONT_NAME        "MS Sans Serif"
#DEFINE DIALOG_LARGEFONT_NAME        "Arial"

