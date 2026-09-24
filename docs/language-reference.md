# Language reference

_Generated from the built-in registry and the control registry. Do not edit by hand; run_
_`REGEN_DOCS=1 npx vitest run tests/shared/languageReference` after changing either._

Statements the compiler accepts are listed in the README. This file covers the two things
you would otherwise have to discover by trial: which functions exist, and which properties
each control has.

## Functions (437)

416 work. 21 are recognised but report why they cannot run, so a
call fails with an explanation rather than "procedure not found".

### Available

| Function | Arguments |
|---|---|
| ABS | 1 |
| ACLASS | 2 |
| ACOPY | 2-5 |
| ACOS | 1 |
| ADATABASES | 1 |
| ADBOBJECTS | 2 |
| ADDBS | 1 |
| ADDPROPERTY | 2-3 |
| ADEL | 2-3 |
| ADIR | 1-4 |
| ADLLS | 1-2 |
| ADOCKSTATE | 1 |
| AELEMENT | 2-3 |
| AEMPTY | 1 |
| AEMPTYNEW | 1 |
| AERROR | 1 |
| AEVENTS | 1-2 |
| AFIELDS | 1-2 |
| AFONT | 1-3 |
| AGETCLASS | 1-7 |
| AGETFILEVERSION | 2 |
| AINS | 2-3 |
| AINSTANCE | 2 |
| ALANGUAGE | 1-2 |
| ALEN | 1-2 |
| ALIAS | 0-1 |
| ALINES | 2+ |
| ALLTRIM | 1+ |
| AMEMBERS | 2-4 |
| AMOUSEOBJ | 1-2 |
| ANETRESOURCES | 1-3 |
| APRINTERS | 1 |
| APROCINFO | 2-3 |
| ASC | 1 |
| ASCAN | 2+ |
| ASELOBJ | 1-2 |
| ASESSIONS | 1 |
| ASIN | 1 |
| ASORT | 1-5 |
| ASQLHANDLES | 1-2 |
| ASTACKINFO | 1 |
| ASUBSCRIPT | 3 |
| AT | 2-3 |
| AT_C | 2-3 |
| ATAGINFO | 1-3 |
| ATAN | 1 |
| ATC | 2-3 |
| ATCC | 2-3 |
| ATCLINE | 2 |
| ATLINE | 2 |
| ATN2 | 2 |
| AUSED | 1-2 |
| AVCXCLASSES | 2 |
| BAR | 0 |
| BARCOUNT | 0-1 |
| BARPROMPT | 1-2 |
| BETWEEN | 3 |
| BINDEVENT | 4-5 |
| BINTOC | 1-2 |
| BITAND | 2+ |
| BITCLEAR | 2+ |
| BITLSHIFT | 2 |
| BITNOT | 1 |
| BITOR | 2+ |
| BITRSHIFT | 2 |
| BITSET | 2+ |
| BITTEST | 2 |
| BITXOR | 2+ |
| BOF | 0-1 |
| CANDIDATE | 0-2 |
| CAPSLOCK | 0-1 |
| CAST | 1-2 |
| CDOW | 1 |
| CDX | 0-2 |
| CEILING | 1 |
| CHR | 1 |
| CHRSAW | 0-1 |
| CHRTRAN | 3 |
| CHRTRANC | 3 |
| CLEARRESULTSET | 0 |
| CMONTH | 1 |
| CNTBAR | 0-1 |
| CNTPAD | 0-1 |
| COL | 0 |
| COMARRAY | 1-2 |
| COMPOBJ | 2 |
| COMPROP | 2-3 |
| COMRETURNERROR | 2 |
| COS | 1 |
| CPCONVERT | 3 |
| CPCURRENT | 0-1 |
| CPDBF | 0-1 |
| CREATEBINARY | 1 |
| CREATEOBJECT | 1+ |
| CREATEOBJECTEX | 1-3 |
| CREATEOFFLINE | 1-2 |
| CTOBIN | 1-2 |
| CTOD | 1 |
| CTOT | 1 |
| CURDIR | 0-1 |
| CURSORGETPROP | 1-2 |
| CURSORSETPROP | 1-3 |
| CURSORTOXML | 2-6 |
| CURVAL | 1-2 |
| DATE | 0-3 |
| DATETIME | 0-6 |
| DAY | 1 |
| DBC | 0 |
| DBF | 0-1 |
| DBGETPROP | 3 |
| DBSETPROP | 4 |
| DBUSED | 1 |
| DEFAULTEXT | 2 |
| DELETED | 0-1 |
| DESCENDING | 0-2 |
| DIFFERENCE | 2 |
| DIRECTORY | 1-2 |
| DISKSPACE | 0-2 |
| DISPLAYPATH | 1-2 |
| DMY | 1 |
| DODEFAULT | 0+ |
| DOW | 1-2 |
| DRIVETYPE | 1 |
| DROPOFFLINE | 1 |
| DTOC | 1-2 |
| DTOR | 1 |
| DTOS | 1 |
| DTOT | 1 |
| EDITSOURCE | 1-4 |
| EMPTY | 1 |
| EOF | 0-1 |
| ERROR | 0 |
| EVALUATE | 1 |
| EVL | 2 |
| EXECSCRIPT | 1+ |
| EXP | 1 |
| FCHSIZE | 2 |
| FCLOSE | 1 |
| FCOUNT | 0-1 |
| FCREATE | 1-2 |
| FDATE | 1-2 |
| FEOF | 1 |
| FERROR | 0 |
| FFLUSH | 1-2 |
| FGETS | 1-2 |
| FIELD | 1-2 |
| FILE | 1-2 |
| FILETOSTR | 1 |
| FILTER | 0-1 |
| FKLABEL | 1 |
| FKMAX | 0 |
| FLDLIST | 0-1 |
| FLOCK | 0-1 |
| FLOOR | 1 |
| FONTMETRIC | 1-4 |
| FOPEN | 1-2 |
| FOR | 0-3 |
| FORCEEXT | 2 |
| FORCEPATH | 2 |
| FOUND | 0-1 |
| FPUTS | 2-3 |
| FREAD | 2 |
| FSEEK | 2-3 |
| FSIZE | 1-2 |
| FTIME | 1 |
| FULLPATH | 1-2 |
| FV | 3 |
| FWRITE | 2-3 |
| GETAUTOINCVALUE | 0-1 |
| GETBAR | 2 |
| GETCOLOR | 0-1 |
| GETCP | 0-3 |
| GETCURSORADAPTER | 1 |
| GETDIR | 0-4 |
| GETENV | 1 |
| GETFILE | 0-5 |
| GETFLDSTATE | 1-2 |
| GETFONT | 0-4 |
| GETKEY | 0 |
| GETNEXTMODIFIED | 1-2 |
| GETOBJECT | 1-2 |
| GETPAD | 2 |
| GETPEM | 2 |
| GETPICT | 0-3 |
| GETPRINTER | 0 |
| GETRESULTSET | 0 |
| GETWORDCOUNT | 1-2 |
| GETWORDNUM | 2-3 |
| GOMONTH | 2 |
| HEADER | 0-1 |
| HOME | 0-1 |
| HOUR | 1 |
| ICASE | 1+ |
| IDXCOLLATE | 0-3 |
| IMESTATUS | 0-1 |
| INDBC | 2 |
| INDEXSEEK | 1-4 |
| INKEY | 0-2 |
| INLIST | 2+ |
| INPUTBOX | 1-5 |
| INSMODE | 0-1 |
| INT | 1 |
| ISALPHA | 1 |
| ISBLANK | 1 |
| ISCOLOR | 0 |
| ISDIGIT | 1 |
| ISEXCLUSIVE | 0-2 |
| ISFLOCKED | 0-1 |
| ISLEADBYTE | 1 |
| ISLOWER | 1 |
| ISMEMOFETCHED | 1-2 |
| ISMOUSE | 0 |
| ISNULL | 1 |
| ISPEN | 0 |
| ISREADONLY | 0-1 |
| ISRLOCKED | 0-2 |
| ISTRANSACTABLE | 1 |
| ISUPPER | 1 |
| JUSTDRIVE | 1 |
| JUSTEXT | 1 |
| JUSTFNAME | 1 |
| JUSTPATH | 1 |
| JUSTSTEM | 1 |
| KEY | 0-2 |
| KEYMATCH | 1-3 |
| LASTKEY | 0 |
| LEFT | 2 |
| LEFTC | 2 |
| LEN | 1 |
| LENC | 1 |
| LIKE | 2 |
| LIKEC | 2 |
| LINENO | 0-1 |
| LOADPICTURE | 0-1 |
| LOCFILE | 1-3 |
| LOCK | 0-2 |
| LOG | 1 |
| LOG10 | 1 |
| LOOKUP | 3-4 |
| LOWER | 1 |
| LTRIM | 1+ |
| LUPDATE | 0-1 |
| MAKETRANSACTABLE | 1 |
| MAX | 2+ |
| MCOL | 0-2 |
| MDOWN | 0 |
| MDX | 1-2 |
| MDY | 1 |
| MEMLINES | 1 |
| MEMORY | 0-1 |
| MENU | 0 |
| MESSAGE | 0-1 |
| MESSAGEBOX | 1-4 |
| MIN | 2+ |
| MINUTE | 1 |
| MLINE | 2-3 |
| MOD | 2 |
| MONTH | 1 |
| MRKBAR | 2 |
| MRKPAD | 2 |
| MROW | 0-2 |
| MSGBOX | 1-4 |
| MTON | 1 |
| MWINDOW | 0-1 |
| NDX | 0-2 |
| NEWOBJECT | 1+ |
| NORMALIZE | 1 |
| NTOM | 1 |
| NUMLOCK | 0-1 |
| NVL | 2 |
| OBJNUM | 1-2 |
| OBJTOCLIENT | 2 |
| OBJVAR | 1 |
| OCCURS | 2 |
| OEMTOANSI | 1 |
| OLDVAL | 1-2 |
| ON | 1-2 |
| ORDER | 0-2 |
| OS | 0-1 |
| PAD | 0 |
| PADC | 2-3 |
| PADL | 2-3 |
| PADR | 2-3 |
| PARAMETERS | 0 |
| PAYMENT | 3 |
| PCOL | 0 |
| PCOUNT | 0 |
| PEMSTATUS | 3 |
| PI | 0 |
| POPUP | 0-1 |
| PRIMARY | 0-2 |
| PRINTSTATUS | 0 |
| PRMBAR | 2 |
| PRMPAD | 2 |
| PROGRAM | 0-1 |
| PROMPT | 0 |
| PROPER | 1 |
| PROW | 0 |
| PRTINFO | 1-2 |
| PUTFILE | 0-3 |
| PV | 3 |
| QUARTER | 0-1 |
| RAISEEVENT | 2+ |
| RAND | 0-1 |
| RAT | 2-3 |
| RATC | 2-3 |
| RATLINE | 2 |
| RDLEVEL | 0 |
| READKEY | 0-1 |
| RECCOUNT | 0-1 |
| RECNO | 0-1 |
| RECSIZE | 0-1 |
| REFRESH | 0-3 |
| RELATION | 1-2 |
| REMOVEPROPERTY | 2 |
| REPLICATE | 2 |
| REQUERY | 0-1 |
| RGB | 3 |
| RGBSCHEME | 1-2 |
| RIGHT | 2 |
| RIGHTC | 2 |
| RLOCK | 0-2 |
| ROUND | 2 |
| ROW | 0 |
| RTOD | 1 |
| RTRIM | 1+ |
| SAVEPICTURE | 2 |
| SCHEME | 1-2 |
| SCOLS | 0 |
| SEC | 1 |
| SECONDS | 0 |
| SEEK | 1-3 |
| SELECT | 0-1 |
| SET | 1-2 |
| SETFLDSTATE | 2-3 |
| SETRESULTSET | 1 |
| SIGN | 1 |
| SIN | 1 |
| SKPBAR | 2 |
| SKPPAD | 2 |
| SOUNDEX | 1 |
| SPACE | 1 |
| SQLCANCEL | 1 |
| SQLCOLUMNS | 2-4 |
| SQLCOMMIT | 1 |
| SQLCONNECT | 0+ |
| SQLDISCONNECT | 0-1 |
| SQLEXEC | 1+ |
| SQLGETPROP | 2 |
| SQLIDLEDISCONNECT | 1 |
| SQLMORERESULTS | 1-2 |
| SQLPREPARE | 2-3 |
| SQLROLLBACK | 1 |
| SQLSETPROP | 2-3 |
| SQLSTRINGCONNECT | 1-2 |
| SQLTABLES | 1-3 |
| SQRT | 1 |
| SROWS | 0 |
| STR | 1-3 |
| STRCONV | 2-4 |
| STREXTRACT | 2-5 |
| STRTOFILE | 2-3 |
| STRTRAN | 2-6 |
| STUFF | 4 |
| STUFFC | 4 |
| SUBSTR | 2-3 |
| SUBSTRC | 2-3 |
| SYS | 1+ |
| SYSMETRIC | 1 |
| TABLEREVERT | 0-2 |
| TABLEUPDATE | 0-3 |
| TAG | 0-3 |
| TAGCOUNT | 0-2 |
| TAGNO | 0-3 |
| TAN | 1 |
| TARGET | 1-2 |
| TEXTMERGE | 1-4 |
| TIME | 0-1 |
| TRANSFORM | 1-2 |
| TRIM | 1+ |
| TTOC | 1-2 |
| TTOD | 1 |
| TXNLEVEL | 0 |
| TXTWIDTH | 1-4 |
| TYPE | 1-2 |
| UNBINDEVENTS | 0-4 |
| UNIQUE | 0-2 |
| UPDATED | 0 |
| UPPER | 1 |
| USED | 0-1 |
| VAL | 1 |
| VARREAD | 0 |
| VARTYPE | 1-2 |
| VERSION | 0-1 |
| WBORDER | 0-1 |
| WCHILD | 0-2 |
| WCOLS | 0-1 |
| WDOCKABLE | 1 |
| WEEK | 1-3 |
| WEXIST | 1 |
| WFONT | 1-2 |
| WLAST | 0-1 |
| WLCOL | 0-1 |
| WLROW | 0-1 |
| WMAXIMUM | 0-1 |
| WMINIMUM | 0-1 |
| WONTOP | 0-1 |
| WOUTPUT | 0-1 |
| WPARENT | 0-1 |
| WREAD | 0-1 |
| WROWS | 0-1 |
| WTITLE | 0-1 |
| WVISIBLE | 1 |
| XMLTOCURSOR | 1-3 |
| XMLUPDATEGRAM | 0-3 |
| YEAR | 1 |

### Recognised but not available yet

| Function | Why |
|---|---|
| ADDTABLESCHEMA | XMLAdapter needs the data engine, which arrives in a later milestone |
| APPLYDIFFGRAM | XMLAdapter needs the data engine, which arrives in a later milestone |
| COMCLASSINFO | what a COM class says about itself is read from its type library, which the addon this runtime talks to COM through does not open; the object itself answers to CREATEOBJECT() and to its own members |
| DDEABORTTRANS | this runtime holds no DDE transaction to abandon |
| DDEADVISE | this runtime holds no DDE conversation to be notified over |
| DDEENABLED | Dynamic Data Exchange is not something this runtime does |
| DDEEXECUTE | this runtime holds no DDE conversation to send a command over |
| DDEINITIATE | Dynamic Data Exchange is a conversation between two Windows programs over the window messages they send each other, and this runtime has no window of that kind to hold one; drive the other program through COM instead |
| DDELASTERROR | this runtime holds no DDE conversation to have failed |
| DDEPOKE | this runtime holds no DDE conversation to send data over |
| DDEREQUEST | this runtime holds no DDE conversation to ask over |
| DDESETOPTION | Dynamic Data Exchange is not something this runtime does |
| DDESETSERVICE | this runtime cannot be a DDE server, because it has no window to answer on |
| DDESETTOPIC | this runtime cannot be a DDE server |
| DDETERMINATE | there is no DDE conversation to end, because this runtime cannot start one |
| EVENTHANDLER | binding to the events a COM object raises needs a connection point of our own for it to call back into, which this runtime does not put up; poll the object, or have it call a program with DO |
| GETINTERFACE | asking an object for another of its interfaces needs the interface's own description from a type library, which this runtime does not read; the automation interface is the one every member is reached through |
| LOADXML | XMLAdapter needs the data engine, which arrives in a later milestone |
| PARSFONT | there is no such Visual FoxPro function, so nothing here implements it; did you mean GETFONT(), WFONT() or FONTMETRIC()? |
| TOCURSOR | XMLAdapter needs the data engine, which arrives in a later milestone |
| TOXML | XMLAdapter needs the data engine, which arrives in a later milestone |

## Control properties

Defaults matter: documents store only values that differ from the default, so what is listed
here is what a saved form leaves out.

### Form

Base class `form`. 111 properties, 34 events.

| Property | Editor | Default |
|---|---|---|
| ActiveControl | text | (empty) |
| ActiveForm | text | .NULL. |
| AllowOutput | boolean | .T. |
| AlwaysOnBottom | boolean | .F. |
| AlwaysOnTop | boolean | .F. |
| Application | text | (empty) |
| AutoCenter | boolean | .F. |
| BackColor | color | 15790320 |
| BaseClass | text | "Form" |
| BindControls | boolean | .T. |
| BorderStyle | enum | 3 |
| BufferMode | enum | 0 |
| Caption | text | "Form" |
| Class | text | "Form" |
| ClassLibrary | text | (empty) |
| ClipControls | boolean | .T. |
| Closable | boolean | .T. |
| ColorSource | enum | 4 |
| Comment | multiline | (empty) |
| ContinuousScroll | boolean | .T. |
| ControlBox | boolean | .T. |
| ControlCount | number | 0 |
| Controls | text | (empty) |
| CURRENTX | number | 0 |
| CURRENTY | number | 0 |
| DataSession | enum | 1 |
| DataSessionID | number | 1 |
| DEClass | text | (empty) |
| DEClassLibrary | text | (empty) |
| DefOLELCID | number | 0 |
| Desktop | boolean | .F. |
| Dockable | number | 0 |
| Docked | boolean | .F. |
| DockPosition | number | -1 |
| DrawMode | enum | 13 |
| DrawStyle | enum | 0 |
| DrawWidth | number | 1 |
| Enabled | boolean | .T. |
| FillColor | number | 0 |
| FillStyle | number | 1 |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| HalfHeightCaption | boolean | .F. |
| Height | number | 250 |
| HelpContextID | number | 0 |
| HScrollSmallChange | number | 10 |
| hWnd | number | 0 |
| Icon | picture | (empty) |
| KeyPreview | boolean | .F. |
| Left | number | 0 |
| LockScreen | boolean | .F. |
| MacDesktop | number | 0 |
| MaxButton | boolean | .T. |
| MaxHeight | number | -1 |
| MaxLeft | number | -1 |
| MaxTop | number | -1 |
| MaxWidth | number | -1 |
| MDIForm | boolean | .F. |
| MinButton | boolean | .T. |
| MinHeight | number | -1 |
| MinWidth | number | -1 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Movable | boolean | .T. |
| Name | text | "Form" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| ReleaseType | enum | 0 |
| RightToLeft | boolean | .F. |
| ScaleMode | enum | 3 |
| ScrollBars | enum | 0 |
| ShowInTaskbar | boolean | .T. |
| ShowTips | boolean | .F. |
| ShowWindow | enum | 0 |
| SizeBox | boolean | .F. |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| Themes | boolean | .T. |
| TitleBar | number | 1 |
| Top | number | 0 |
| ViewPortHeight | number | 250 |
| ViewPortLeft | number | 0 |
| ViewPortTop | number | 0 |
| ViewPortWidth | number | 375 |
| Visible | boolean | .F. |
| VScrollSmallChange | number | 10 |
| WhatsThisButton | boolean | .F. |
| WhatsThisHelp | boolean | .F. |
| WhatsThisHelpID | number | -1 |
| Width | number | 375 |
| WindowState | enum | 0 |
| WindowType | enum | 0 |
| ZoomBox | boolean | .F. |

**Events**: Activate, AfterDock, BeforeDock, Click, DblClick, Deactivate, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), GotFocus, Init, KeyPress(nKeyCode, nShiftAltCtrl), Load, LostFocus, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), Moved, OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), Paint, QueryUnload, Resize, RightClick, Scrolled(nDirection), UnDock, Unload

### CheckBox

Base class `checkbox`. 68 properties, 31 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 0 |
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoSize | boolean | .F. |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Checkbox" |
| Caption | text | "Check" |
| Centered | boolean | .F. |
| Class | text | "Checkbox" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DisabledPicture | picture | (empty) |
| DownPicture | picture | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Check" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| PictureMargin | number | 0 |
| PicturePosition | enum | 13 |
| PictureSpacing | number | 0 |
| ReadOnly | boolean | .F. |
| RightToLeft | boolean | .F. |
| SpecialEffect | number | 0 |
| StatusBarText | text | (empty) |
| Style | enum | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |
| WordWrap | boolean | .F. |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, InteractiveChange, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RightClick, UIEnable(lEnable), Valid, When

### Collection

Base class `collection`. 11 properties, 3 events.

| Property | Editor | Default |
|---|---|---|
| Application | text | (empty) |
| BaseClass | text | "Collection" |
| Class | text | "Collection" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| Count | number | 0 |
| KeySort | number | 0 |
| Name | text | "Collection" |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Tag | text | (empty) |

**Events**: Destroy, Error(nError, cMethod, nLine), Init

### Column

Base class `column`. 59 properties, 9 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 3 |
| Application | text | (empty) |
| BackColor | color | 16777215 |
| BaseClass | text | "Column" |
| Bound | boolean | .T. |
| Class | text | "Column" |
| ClassLibrary | text | (empty) |
| ColumnOrder | number | 0 |
| Comment | multiline | (empty) |
| ControlCount | number | 0 |
| Controls | text | (empty) |
| ControlSource | expression | (empty) |
| CurrentControl | text | (empty) |
| DynamicAlignment | expression | (empty) |
| DynamicBackColor | expression | (empty) |
| DynamicCurrentControl | expression | (empty) |
| DynamicFontBold | expression | (empty) |
| DynamicFontItalic | expression | (empty) |
| DynamicFontName | expression | (empty) |
| DynamicFontOutline | expression | (empty) |
| DynamicFontShadow | expression | (empty) |
| DynamicFontSize | expression | (empty) |
| DynamicFontStrikethru | expression | (empty) |
| DynamicFontUnderline | expression | (empty) |
| DynamicForeColor | expression | (empty) |
| DynamicInputMask | expression | (empty) |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Format | text | (empty) |
| HeaderClass | text | (empty) |
| HeaderClassLibrary | text | (empty) |
| InputMask | text | (empty) |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Movable | boolean | .T. |
| Name | text | "Column" |
| Objects | text | (empty) |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| ReadOnly | boolean | .F. |
| Resizable | boolean | .T. |
| SelectOnEntry | boolean | .T. |
| Sparse | boolean | .T. |
| StatusBarText | text | (empty) |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| Visible | boolean | .T. |
| Width | number | 75 |

**Events**: Destroy, Error(nError, cMethod, nLine), Init, MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), Moved, Resize

### ComboBox

Base class `combobox`. 109 properties, 36 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | number | 0 |
| Anchor | number | 0 |
| Application | text | (empty) |
| BackColor | color | 16777215 |
| BaseClass | text | "Combobox" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 1 |
| BoundColumn | number | 1 |
| BoundTo | boolean | .F. |
| Class | text | "Combobox" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 2 |
| ColorSource | number | 4 |
| ColumnCount | number | 0 |
| ColumnLines | boolean | .T. |
| ColumnWidths | text | (empty) |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DISABLEDITEMBACKCOLOR | number | 16777215 |
| DISABLEDITEMFORECOLOR | number | 7171437 |
| DisplayCount | number | 0 |
| DisplayValue | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FirstElement | number | 1 |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Format | text | (empty) |
| Height | number | 24 |
| HelpContextID | number | 0 |
| HideSelection | boolean | .T. |
| IMEMode | enum | 0 |
| IncrementalSearch | boolean | .T. |
| InputMask | text | (empty) |
| ITEMBACKCOLOR | number | 16777215 |
| ItemData | number | 0 |
| ITEMFORECOLOR | number | 0 |
| ItemIDData | number | 0 |
| ItemTips | boolean | .F. |
| Left | number | 0 |
| List | text | (empty) |
| ListCount | number | 0 |
| ListIndex | number | 0 |
| ListItem | text | (empty) |
| ListItemID | number | 0 |
| Margin | number | 2 |
| MaxLength | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Combo" |
| NewIndex | number | 0 |
| NewItemID | number | 0 |
| NullDisplay | text | (empty) |
| NumberOfElements | number | 0 |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| OLEDropTextInsertion | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| PictureSelectionDisplay | enum | 0 |
| ReadOnly | boolean | .F. |
| RightToLeft | boolean | .F. |
| RowSource | expression | (empty) |
| RowSourceType | enum | 0 |
| Selected | boolean | .F. |
| SelectedBackColor | number | 13924352 |
| SelectedForeColor | number | 16777215 |
| SelectedID | boolean | .F. |
| SelectedItemBackColor | color | 13924352 |
| SelectedItemForeColor | color | 16777215 |
| SelectOnEntry | boolean | .F. |
| SelLength | number | 0 |
| SelStart | number | 0 |
| SelText | text | (empty) |
| Sorted | boolean | .F. |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| Style | enum | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Text | text | "              " |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| TopIndex | number | 1 |
| TopItemID | number | -1 |
| Value | text | (empty) |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DownClick, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), DropDown, Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, InteractiveChange, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RangeHigh, RangeLow, RightClick, UIEnable(lEnable), UpClick, Valid, When

### CommandButton

Base class `commandbutton`. 66 properties, 28 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | number | 2 |
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoSize | boolean | .F. |
| BackColor | color | 15790320 |
| BaseClass | text | "Commandbutton" |
| Cancel | boolean | .F. |
| Caption | text | "Command" |
| Class | text | "Commandbutton" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| Default | boolean | .F. |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DisabledPicture | picture | (empty) |
| DownPicture | picture | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Command" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| PictureMargin | number | 0 |
| PicturePosition | enum | 13 |
| PictureSpacing | number | 0 |
| RightToLeft | boolean | .F. |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| Style | number | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| VisualEffect | number | 0 |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |
| WordWrap | boolean | .F. |

**Events**: Click, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick, UIEnable(lEnable), Valid, When

### CommandGroup

Base class `commandgroup`. 46 properties, 28 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoSize | boolean | .F. |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Commandgroup" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 1 |
| ButtonCount | number | 0 |
| Buttons | text | (empty) |
| Class | text | "Commandgroup" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MemberClass | text | (empty) |
| MemberClassLibrary | text | (empty) |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Commandgroup" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 10 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, Init, InteractiveChange, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RightClick, UIEnable(lEnable), Valid, When

### Container

Base class `container`. 44 properties, 26 events.

| Property | Editor | Default |
|---|---|---|
| ActiveControl | text | .NULL. |
| Anchor | number | 0 |
| Application | text | (empty) |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Container" |
| BorderColor | number | 6579300 |
| BorderWidth | number | 1 |
| Class | text | "Container" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlCount | number | 0 |
| Controls | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| ForeColor | color | 0 |
| Height | number | 75 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Container" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| SpecialEffect | enum | 2 |
| StatusBarText | text | (empty) |
| Style | number | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 75 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), GotFocus, Init, LostFocus, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), Moved, OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), Resize, RightClick, UIEnable(lEnable)

### Custom

Base class `custom`. 19 properties, 3 events.

| Property | Editor | Default |
|---|---|---|
| Application | text | (empty) |
| BaseClass | text | "Custom" |
| Class | text | "Custom" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| ControlCount | number | 0 |
| Controls | text | (empty) |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| Name | text | "Custom" |
| Objects | text | (empty) |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | text | (empty) |
| Tag | text | (empty) |
| Top | number | 0 |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Destroy, Error(nError, cMethod, nLine), Init

### EditBox

Base class `editbox`. 79 properties, 31 events.

| Property | Editor | Default |
|---|---|---|
| AddLineFeeds | boolean | .T. |
| Alignment | enum | 0 |
| AllowTabs | boolean | .F. |
| Anchor | number | 0 |
| Application | text | (empty) |
| BackColor | color | 16777215 |
| BackStyle | number | 1 |
| BaseClass | text | "Editbox" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 1 |
| Class | text | "Editbox" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 2 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| EnableHyperlinks | boolean | .F. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Format | text | (empty) |
| Height | number | 75 |
| HelpContextID | number | 0 |
| HideSelection | boolean | .T. |
| IMEMode | enum | 0 |
| IntegralHeight | boolean | .F. |
| Left | number | 0 |
| Margin | number | 2 |
| MaxLength | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Edit" |
| NullDisplay | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| OLEDropTextInsertion | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| PasswordChar | text | (empty) |
| ReadOnly | boolean | .F. |
| RightToLeft | boolean | .F. |
| ScrollBars | enum | 2 |
| SelectedBackColor | color | 13924352 |
| SelectedForeColor | color | 16777215 |
| SelectOnEntry | boolean | .F. |
| SelLength | number | 0 |
| SelStart | number | 0 |
| SelText | text | (empty) |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Text | text | (empty) |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | text | (empty) |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, InteractiveChange, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RightClick, UIEnable(lEnable), Valid, When

### Grid

Base class `grid`. 93 properties, 31 events.

| Property | Editor | Default |
|---|---|---|
| ActiveColumn | number | 0 |
| ActiveRow | number | 0 |
| AllowAddNew | boolean | .F. |
| AllowAutoColumnFit | number | 0 |
| AllowCellSelection | boolean | .T. |
| AllowHeaderSizing | boolean | .T. |
| AllowRowSizing | boolean | .T. |
| Anchor | number | 0 |
| Application | text | (empty) |
| BackColor | color | 16777215 |
| BaseClass | text | "Grid" |
| ChildOrder | expression | (empty) |
| Class | text | "Grid" |
| ClassLibrary | text | (empty) |
| ColumnCount | number | -1 |
| Columns | text | (empty) |
| Comment | multiline | (empty) |
| DeleteMark | boolean | .T. |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| GridLineColor | color | 0 |
| GridLines | enum | 3 |
| GridLineWidth | number | 1 |
| HeaderHeight | number | 19 |
| Height | number | 200 |
| HelpContextID | number | 0 |
| Highlight | boolean | .T. |
| HighlightBackColor | color | 13924352 |
| HighlightForeColor | color | 16777215 |
| HighlightRow | boolean | .T. |
| HighlightRowLineWidth | number | 1 |
| HighlightStyle | enum | 0 |
| Left | number | 0 |
| LeftColumn | number | 1 |
| LinkMaster | expression | (empty) |
| LockColumns | number | 0 |
| LockColumnsLeft | number | 0 |
| MemberClass | text | (empty) |
| MemberClassLibrary | text | (empty) |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Grid" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Optimize | boolean | .F. |
| Panel | number | 1 |
| PanelLink | boolean | .T. |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Partition | number | 0 |
| ReadOnly | boolean | .F. |
| RecordMark | boolean | .T. |
| RecordSource | expression | (empty) |
| RecordSourceType | enum | 1 |
| RelationalExpr | expression | (empty) |
| RelativeColumn | number | 0 |
| RelativeRow | number | 0 |
| RightToLeft | boolean | .F. |
| RowColChange | number | 0 |
| RowHeight | number | 18 |
| ScrollBars | enum | 3 |
| SelectedItemBackColor | color | 13924352 |
| SelectedItemForeColor | color | 16777215 |
| SplitBar | boolean | .T. |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | number | 0 |
| View | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 320 |

**Events**: AfterRowColChange(nColIndex), BeforeRowColChange(nColIndex), Click, DblClick, Deleted, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, KeyPress(nKeyCode, nShiftAltCtrl), MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), Moved, OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), Resize, RightClick, Scrolled(nDirection), UIEnable(lEnable), Valid, When

### Header

Base class `header`. 30 properties, 13 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 0 |
| Application | text | (empty) |
| BackColor | color | 15790320 |
| BaseClass | text | "Header" |
| Caption | text | "Header" |
| Class | text | "Header" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Header" |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| StatusBarText | text | (empty) |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| WordWrap | boolean | .F. |

**Events**: Click, DblClick, Destroy, Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), RightClick

### Hyperlink

Base class `hyperlink`. 9 properties, 3 events.

| Property | Editor | Default |
|---|---|---|
| Application | text | (empty) |
| BaseClass | text | "Hyperlink" |
| Class | text | "Hyperlink" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| Name | text | "Hyperlink" |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Tag | text | (empty) |

**Events**: Destroy, Error(nError, cMethod, nLine), Init

### Image

Base class `image`. 38 properties, 22 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| BackStyle | enum | 1 |
| BaseClass | text | "Image" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 0 |
| Class | text | "Image" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Image" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| PictureVal | text | (empty) |
| RotateFlip | enum | 0 |
| StatusBarText | text | (empty) |
| Stretch | enum | 0 |
| Tag | text | (empty) |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick, UIEnable(lEnable)

### Label

Base class `label`. 56 properties, 22 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 0 |
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoSize | boolean | .F. |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Label" |
| BorderStyle | enum | 0 |
| Caption | text | "Label" |
| Class | text | "Label" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Label" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| RightToLeft | boolean | .F. |
| Rotation | number | 0 |
| StatusBarText | text | (empty) |
| Style | number | 0 |
| TabIndex | number | 0 |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |
| WordWrap | boolean | .F. |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick, UIEnable(lEnable)

### Line

Base class `line`. 37 properties, 22 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| BaseClass | text | "Line" |
| BorderColor | color | 6579300 |
| BorderStyle | enum | 1 |
| BorderWidth | number | 1 |
| Class | text | "Line" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| DrawMode | enum | 13 |
| Enabled | boolean | .T. |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| LineSlant | enum | "\" |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Line" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| PolyPoints | text | (empty) |
| Rotation | number | 0 |
| StatusBarText | text | (empty) |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick, UIEnable(lEnable)

### ListBox

Base class `listbox`. 91 properties, 34 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoHideScrollBar | number | 0 |
| BaseClass | text | "Listbox" |
| BorderColor | number | 6579300 |
| BoundColumn | number | 1 |
| BoundTo | boolean | .F. |
| Class | text | "Listbox" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| ColumnCount | number | 0 |
| ColumnLines | boolean | .T. |
| ColumnWidths | text | (empty) |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DisabledBackColor | color | 16777215 |
| DisabledForeColor | color | 7171437 |
| DISABLEDITEMBACKCOLOR | number | 16777215 |
| DISABLEDITEMFORECOLOR | number | 7171437 |
| DisplayValue | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FirstElement | number | 1 |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| Height | number | 170 |
| HelpContextID | number | 0 |
| IncrementalSearch | boolean | .T. |
| IntegralHeight | boolean | .F. |
| ITEMBACKCOLOR | number | 16777215 |
| ItemData | number | 0 |
| ITEMFORECOLOR | number | 0 |
| ItemIDData | number | 0 |
| ItemTips | boolean | .F. |
| Left | number | 0 |
| List | text | (empty) |
| ListCount | number | 0 |
| ListIndex | number | 0 |
| ListItem | text | (empty) |
| ListItemID | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| MoverBars | boolean | .F. |
| MultiSelect | boolean | .F. |
| Name | text | "List" |
| NewIndex | number | 0 |
| NewItemID | number | 0 |
| NullDisplay | text | (empty) |
| NumberOfElements | number | 0 |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| RightToLeft | boolean | .F. |
| RowSource | expression | (empty) |
| RowSourceType | enum | 0 |
| Selected | boolean | .F. |
| SelectedID | boolean | .F. |
| SelectedItemBackColor | color | 13924352 |
| SelectedItemForeColor | color | 16777215 |
| Sorted | boolean | .F. |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| TopIndex | number | 1 |
| TopItemID | number | -1 |
| Value | text | (empty) |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, InteractiveChange, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), OnMoveItem(nSource, nShift, nCurrentIndex, nMoveBy), ProgrammaticChange, RangeHigh, RangeLow, RightClick, UIEnable(lEnable), Valid, When

### OleBoundControl

Base class `oleboundcontrol`. 37 properties, 10 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoActivate | enum | 2 |
| AutoSize | boolean | .F. |
| AutoVerbMenu | boolean | .T. |
| BaseClass | text | "Oleboundcontrol" |
| Class | text | "Oleboundcontrol" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| ControlSource | text | (empty) |
| DocumentFile | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| Height | number | 17 |
| HelpContextID | number | 0 |
| HostName | text | (empty) |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | (empty) |
| OleClass | text | (empty) |
| OLELCID | number | 1033 |
| OLETypeAllowed | enum | -1 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Sizable | boolean | .T. |
| StatusBarText | text | (empty) |
| Stretch | number | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), GotFocus, Init, LostFocus, Moved, Resize, UIEnable(lEnable)

### OleControl

Base class `olecontrol`. 34 properties, 10 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoActivate | enum | 2 |
| AutoSize | boolean | .F. |
| AutoVerbMenu | boolean | .F. |
| BaseClass | text | "Olecontrol" |
| Class | text | "Olecontrol" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| DocumentFile | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| Height | number | 17 |
| HelpContextID | number | 0 |
| HostName | text | (empty) |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | (empty) |
| OleClass | text | (empty) |
| OLELCID | number | 1033 |
| OLETypeAllowed | enum | 1 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Sizable | boolean | .T. |
| Stretch | number | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | 0 |
| Width | number | 100 |

**Events**: Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), GotFocus, Init, LostFocus, Moved, Resize, UIEnable(lEnable)

### OptionButton

Base class `optionbutton`. 66 properties, 29 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 0 |
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoSize | boolean | .F. |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Optionbutton" |
| Caption | text | "Option" |
| Class | text | "Optionbutton" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | text | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DisabledPicture | picture | (empty) |
| DownPicture | picture | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Height | number | 16 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Option" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| PictureMargin | number | 0 |
| PicturePosition | enum | 13 |
| PictureSpacing | number | 0 |
| RightToLeft | boolean | .F. |
| SpecialEffect | number | 0 |
| StatusBarText | text | (empty) |
| Style | enum | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 10 |
| WordWrap | boolean | .F. |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick, UIEnable(lEnable), Valid, When

### OptionGroup

Base class `optiongroup`. 46 properties, 28 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoSize | boolean | .F. |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Optiongroup" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 1 |
| ButtonCount | number | 0 |
| Buttons | text | (empty) |
| Class | text | "Optiongroup" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| Height | number | 15 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MemberClass | text | (empty) |
| MemberClassLibrary | text | (empty) |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Optiongroup" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 10 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, Init, InteractiveChange, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RightClick, UIEnable(lEnable), Valid, When

### Page

Base class `page`. 45 properties, 23 events.

| Property | Editor | Default |
|---|---|---|
| ActiveControl | text | .NULL. |
| Application | text | (empty) |
| BackColor | color | 15790320 |
| BackStyle | number | 1 |
| BaseClass | text | "Page" |
| Caption | text | "Page" |
| Class | text | "Page" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlCount | number | 0 |
| Controls | text | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| HelpContextID | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Page" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| PageOrder | number | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Picture | picture | (empty) |
| StatusBarText | text | (empty) |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| WhatsThisHelpID | number | -1 |

**Events**: Activate, Click, DblClick, Deactivate, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick

### PageFrame

Base class `pageframe`. 49 properties, 24 events.

| Property | Editor | Default |
|---|---|---|
| ActivePage | number | 1 |
| Anchor | number | 0 |
| Application | text | (empty) |
| BaseClass | text | "Pageframe" |
| BorderColor | number | 6579300 |
| BorderWidth | number | 1 |
| Class | text | "Pageframe" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| Height | number | 250 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MemberClass | text | (empty) |
| MemberClassLibrary | text | (empty) |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Pageframe" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| PageCount | number | 0 |
| PageHeight | number | 224 |
| Pages | text | (empty) |
| PageWidth | number | 371 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| RightToLeft | boolean | .T. |
| SpecialEffect | number | 0 |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| TabOrientation | enum | 0 |
| Tabs | boolean | .T. |
| TabStop | boolean | .T. |
| TabStretch | number | 1 |
| TabStyle | enum | 0 |
| Tag | text | (empty) |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 375 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), Moved, OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), Resize, RightClick, UIEnable(lEnable)

### Separator

Base class `separator`. 12 properties, 3 events.

| Property | Editor | Default |
|---|---|---|
| Application | text | (empty) |
| BaseClass | text | "Separator" |
| Class | text | "Separator" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| Enabled | boolean | .T. |
| Name | text | "Separator" |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Style | number | 0 |
| Tag | text | (empty) |
| Visible | boolean | .T. |

**Events**: Destroy, Error(nError, cMethod, nLine), Init

### Session

Base class `session`. 11 properties, 3 events.

| Property | Editor | Default |
|---|---|---|
| Application | text | (empty) |
| BaseClass | text | "Session" |
| Class | text | "Session" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| DataSession | number | 2 |
| DataSessionID | text | .NULL. |
| Name | text | "Session" |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Tag | text | (empty) |

**Events**: Destroy, Error(nError, cMethod, nLine), Init

### Shape

Base class `shape`. 44 properties, 22 events.

| Property | Editor | Default |
|---|---|---|
| Anchor | number | 0 |
| Application | text | (empty) |
| BackColor | color | 15790320 |
| BackStyle | enum | 1 |
| BaseClass | text | "Shape" |
| BorderColor | color | 6579300 |
| BorderStyle | enum | 1 |
| BorderWidth | number | 1 |
| Class | text | "Shape" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| Curvature | number | 0 |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| DrawMode | enum | 13 |
| Enabled | boolean | .T. |
| FillColor | color | 0 |
| FillStyle | enum | 1 |
| Height | number | 17 |
| HelpContextID | number | 0 |
| Left | number | 0 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Shape" |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| PolyPoints | text | (empty) |
| Rotation | number | 0 |
| SpecialEffect | enum | 1 |
| StatusBarText | text | (empty) |
| Style | number | 0 |
| Tag | text | (empty) |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), RightClick, UIEnable(lEnable)

### Spinner

Base class `spinner`. 76 properties, 35 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 1 |
| Anchor | number | 0 |
| Application | text | (empty) |
| BackColor | color | 16777215 |
| BaseClass | text | "Spinner" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 1 |
| Class | text | "Spinner" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Format | text | (empty) |
| Height | number | 24 |
| HelpContextID | number | 0 |
| HideSelection | boolean | .T. |
| Increment | number | 1 |
| InputMask | text | (empty) |
| KeyboardHighValue | number | 2147483647 |
| KeyboardLowValue | number | -2147483647 |
| Left | number | 0 |
| Margin | number | 2 |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Spinner" |
| NullDisplay | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| OLEDropTextInsertion | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| ReadOnly | boolean | .F. |
| RightToLeft | boolean | .F. |
| SelectedBackColor | number | 13924352 |
| SelectedForeColor | number | 16777215 |
| SelectOnEntry | boolean | .F. |
| SelLength | number | 0 |
| SelStart | number | 0 |
| SelText | text | (empty) |
| SpecialEffect | enum | 0 |
| SpinnerHighValue | number | 2147483647 |
| SpinnerLowValue | number | -2147483647 |
| StatusBarText | text | (empty) |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Text | text | "         0" |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DownClick, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, InteractiveChange, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RangeHigh, RangeLow, RightClick, UIEnable(lEnable), UpClick, Valid, When

### TextBox

Base class `textbox`. 89 properties, 33 events.

| Property | Editor | Default |
|---|---|---|
| Alignment | enum | 3 |
| Anchor | number | 0 |
| Application | text | (empty) |
| AutoComplete | number | 0 |
| AutoCompSource | text | (empty) |
| AutoCompTable | text | (empty) |
| BackColor | color | 16777215 |
| BackStyle | number | 1 |
| BaseClass | text | "Textbox" |
| BorderColor | number | 6579300 |
| BorderStyle | enum | 1 |
| Century | enum | 1 |
| Class | text | "Textbox" |
| ClassLibrary | text | (empty) |
| ColorScheme | number | 1 |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlSource | expression | (empty) |
| DateFormat | enum | 0 |
| DateMark | text | (empty) |
| DisabledBackColor | color | 15790320 |
| DisabledForeColor | color | 7171437 |
| DragIcon | picture | (empty) |
| DragMode | enum | 0 |
| Enabled | boolean | .T. |
| EnableHyperlinks | boolean | .F. |
| FontBold | boolean | .F. |
| FontCharSet | number | 1 |
| FONTCONDENSE | boolean | .F. |
| FONTEXTEND | boolean | .F. |
| FontItalic | boolean | .F. |
| FontName | font | "Arial" |
| FontOutline | boolean | .F. |
| FontShadow | boolean | .F. |
| FontSize | number | 9 |
| FontStrikethru | boolean | .F. |
| FontUnderline | boolean | .F. |
| ForeColor | color | 0 |
| Format | text | (empty) |
| Height | number | 21 |
| HelpContextID | number | 0 |
| HideSelection | boolean | .T. |
| Hours | enum | 0 |
| IMEMode | enum | 0 |
| InputMask | text | (empty) |
| IntegralHeight | boolean | .F. |
| Left | number | 0 |
| Margin | number | 2 |
| MaxLength | number | 0 |
| MemoWindow | text | (empty) |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Name | text | "Text" |
| NullDisplay | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| OLEDropTextInsertion | enum | 0 |
| OpenWindow | boolean | .F. |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| PasswordChar | text | (empty) |
| ReadOnly | boolean | .F. |
| RightToLeft | boolean | .F. |
| Seconds | enum | 2 |
| SelectedBackColor | color | 13924352 |
| SelectedForeColor | color | 16777215 |
| SelectOnEntry | boolean | .F. |
| SelLength | number | 0 |
| SelStart | number | 0 |
| SelText | text | (empty) |
| SpecialEffect | enum | 0 |
| StatusBarText | text | (empty) |
| StrictDateEntry | enum | 1 |
| Style | number | 0 |
| TabIndex | number | 0 |
| TabStop | boolean | .T. |
| Tag | text | (empty) |
| TerminateRead | boolean | .F. |
| Text | text | "                 " |
| Themes | boolean | .T. |
| ToolTipText | text | (empty) |
| Top | number | 0 |
| Value | text | (empty) |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 100 |

**Events**: Click, DblClick, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), ErrorMessage, GotFocus, Init, InteractiveChange, KeyPress(nKeyCode, nShiftAltCtrl), LostFocus, Message, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseEnter(nButton, nShift, nXCoord, nYCoord), MouseLeave(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), ProgrammaticChange, RangeHigh, RangeLow, RightClick, UIEnable(lEnable), Valid, When

### Timer

Base class `timer`. 15 properties, 4 events.

| Property | Editor | Default |
|---|---|---|
| Application | text | (empty) |
| BaseClass | text | "Timer" |
| Class | text | "Timer" |
| ClassLibrary | text | (empty) |
| Comment | multiline | (empty) |
| Enabled | boolean | .T. |
| Height | number | 17 |
| Interval | number | 0 |
| Left | number | 0 |
| Name | text | "Timer" |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| Tag | text | (empty) |
| Top | number | 0 |
| Width | number | 100 |

**Events**: Destroy, Error(nError, cMethod, nLine), Init, Timer

### Toolbar

Base class `toolbar`. 46 properties, 27 events.

| Property | Editor | Default |
|---|---|---|
| ActiveControl | text | .NULL. |
| Application | text | (empty) |
| BackColor | color | 15790320 |
| BaseClass | text | "Toolbar" |
| Caption | text | "Toolbar" |
| Class | text | "Toolbar" |
| ClassLibrary | text | (empty) |
| ColorSource | number | 4 |
| Comment | multiline | (empty) |
| ControlBox | boolean | .T. |
| ControlCount | number | 0 |
| Controls | text | (empty) |
| DataSession | number | 1 |
| DataSessionID | number | 1 |
| Docked | boolean | .F. |
| DockPosition | enum | -1 |
| Enabled | boolean | .T. |
| ForeColor | color | 0 |
| Height | number | 15 |
| HelpContextID | number | 0 |
| hWnd | number | 0 |
| KeyPreview | boolean | .F. |
| Left | number | 0 |
| LockScreen | boolean | .F. |
| MouseIcon | picture | (empty) |
| MousePointer | number | 0 |
| Movable | boolean | .T. |
| Name | text | "Toolbar" |
| Objects | text | (empty) |
| OLEDragMode | enum | 0 |
| OLEDragPicture | picture | (empty) |
| OLEDropEffects | number | 3 |
| OLEDropHasData | number | -1 |
| OLEDropMode | enum | 0 |
| Parent | text | (empty) |
| ParentClass | text | (empty) |
| ScaleMode | number | 3 |
| ShowTips | boolean | .T. |
| ShowWindow | number | 0 |
| Sizable | boolean | .T. |
| Tag | text | (empty) |
| Themes | boolean | .T. |
| Top | number | 0 |
| Visible | boolean | .T. |
| WhatsThisHelpID | number | -1 |
| Width | number | 33 |

**Events**: Activate, AfterDock, BeforeDock(nLocation), Click, DblClick, Deactivate, Destroy, DragDrop(oSource, nXCoord, nYCoord), DragOver(oSource, nXCoord, nYCoord, nState), Error(nError, cMethod, nLine), Init, MiddleClick, MouseDown(nButton, nShift, nXCoord, nYCoord), MouseMove(nButton, nShift, nXCoord, nYCoord), MouseUp(nButton, nShift, nXCoord, nYCoord), MouseWheel(nDirection, nShift, nXCoord, nYCoord), Moved, OLECompleteDrag(nEffect), OLEDragDrop(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord), OLEDragOver(oDataObject, nEffect, nButton, nShift, nXCoord, nYCoord, nState), OLEGiveFeedback(nEffect, eMouseCursor), OLESetData(oDataObject, eFormat), OLEStartDrag(oDataObject, nEffect), Paint, Resize, RightClick, UnDock

