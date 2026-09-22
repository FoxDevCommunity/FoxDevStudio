* _REPORTS.H

* Header file for _REPORTS.VCX classes

* common dialog flag constants and results for ShowFont:
#DEFINE cdlCFScreenFonts                        0x1
#DEFINE cdlCFANSIOnly                           0x400
#DEFINE cdlCFForceFontExist                     0x10000
#DEFINE cdlCFNoStyleSel	                        0x100000
#DEFINE cdlCFFixedPitchOnly	                    0x4000	

* common dialog constants for ShowOpen/ShowSave:
#DEFINE cdlOFNPathMustExist                     0x800	
#DEFINE cdlOFNNoChangeDir                       0x8
#DEFINE cdlOFNHideReadOnly                      0x4
#DEFINE cdlOFNExplorer                          0x80000	
#DEFINE cdlOFNFileMustExist                     0x1000	
#DEFINE cdlOFNOverwritePrompt                   0x2
#DEFINE cdlOFNNoReadOnlyReturn                  0x8000

#DEFINE VERTICAL_SCROLLBAR_WIDTH                5

#DEFINE SHOWTEXT_TEXT_EDITOR_LOC                "Text Viewer"

#DEFINE WINDOWSTATE_MAXIMIZED                   2
#DEFINE WINDOWSTATE_NORM                        0

* _dialog fonts
#DEFINE SYSTEM_LARGEFONTS            FONTMETRIC(1, 'MS Sans Serif', 8, '') # 13 OR ;
                                     FONTMETRIC(4, 'MS Sans Serif', 8, '') # 2 OR ;
                                     FONTMETRIC(6, 'MS Sans Serif', 8, '') # 5 OR ;
                                     FONTMETRIC(7, 'MS Sans Serif', 8, '') # 11
                                     
#DEFINE DIALOG_SMALLFONT_NAME        "MS Sans Serif"
#DEFINE DIALOG_LARGEFONT_NAME        "Arial"

* destinations:
#DEFINE OUTPUT_PRINT_REPORT_LOC                "Print report"
#DEFINE OUTPUT_PRINT_LIST_LOC                  "Print list from table"
#DEFINE OUTPUT_SCREEN_LOC                      "Preview"
#DEFINE OUTPUT_TEXTFILE_LOC                    "Text file"
#DEFINE OUTPUT_HTMLFILE_LOC                    "HTML file"
#DEFINE OUTPUT_PRINTFILE_LOC                   "Print-image file"
#DEFINE OUTPUT_EXPORT_LOC                      "Export table"

* screen options:
#DEFINE OUTPUT_SCREEN_PREVIEW_LOC              "Report Preview"
#DEFINE OUTPUT_SCREEN_GRAPHICAL_LOC            "Graphical report preview"
#DEFINE OUTPUT_SCREEN_ASCII_LOC                "Text report preview"
#DEFINE OUTPUT_SCREEN_BROWSE_LOC               "Browse view of table"
#DEFINE OUTPUT_SCREEN_LIST_LOC                 "Simple list view of table"

* HTML options:
#DEFINE OUTPUT_HTML_FILEONLY_LOC               "Generate only, no display"
#DEFINE OUTPUT_HTML_VIEWSOURCE_LOC             "View generated source"
#DEFINE OUTPUT_HTML_WEBVIEW_LOC                "View output in web browser"

#DEFINE OUTPUT_KEY_FOR_TOOLBAR_LOC             "Press Ctrl-R to retrieve Report Preview Toolbar if necessary..."

* print options:
#DEFINE OUTPUT_PRINT_OPTIONS_WINDEFAULT_LOC    "Windows default printer"
#DEFINE OUTPUT_PRINT_OPTIONS_VFPDEFAULT_LOC    "VFP default printer"
#DEFINE OUTPUT_PRINT_OPTIONS_SETVFPDEFAULT_LOC "Select and use new VFP default"

#DEFINE OUTPUT_DESTINATION_TEXTFILE_LOC        "Pick a destination text filename"
#DEFINE OUTPUT_SOURCE_REPORT_LOC               "Pick a report"

#DEFINE OUTPUT_REPORT_NOT_FOUND_LOC            "Report not found!"

#DEFINE OUTPUT_REPORT_OR_DATASOURCE_REQUIRED_LOC "This dialog needs report or table datasource information."
#DEFINE MB_ICONEXCLAMATION                     48

* export options

#DEFINE OUTPUT_EXPORT_EXCEL97   "Excel 97"
#DEFINE OUTPUT_EXPORT_EXCEL5    "Excel 5.0"
#DEFINE OUTPUT_EXPORT_EXCEL2    "Excel 2.0"
#DEFINE OUTPUT_EXPORT_FOX2X     "Fox 2.X"
#DEFINE OUTPUT_EXPORT_FOXPLUS   "Xbase"
#DEFINE OUTPUT_EXPORT_FIXEDLEN  "Fixed Length"
#DEFINE OUTPUT_EXPORT_DELIMITED "Delimited"
#DEFINE OUTPUT_EXPORT_LOTUS2    "Lotus 2.x"
#DEFINE OUTPUT_EXPORT_DIF       "Data Interchange Format"
#DEFINE OUTPUT_EXPORT_SYMPHONY  "Symphony 1.10"
#DEFINE OUTPUT_EXPORT_CSV       "Separated Values"


