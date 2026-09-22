* The system variables ref_sysvars.prg does not reach: what each one starts as, and what a
* program gets back when it sets one.
*
* Visual FoxPro spells the tool variables as full paths into wherever it happens to be
* installed, so what a golden can hold is the file name and not the directory - the job each one
* names is the same on every machine, the place it was installed to is not.

* --- the programs the development environment runs for a job
? UPPER(JUSTFNAME(_BROWSER)), UPPER(JUSTFNAME(_BUILDER)), UPPER(JUSTFNAME(_CODESENSE))
? UPPER(JUSTFNAME(_CONVERTER)), UPPER(JUSTFNAME(_FOXCODE)), UPPER(JUSTFNAME(_FOXREF))
? UPPER(JUSTFNAME(_FoxTask)), UPPER(JUSTFNAME(_GALLERY)), UPPER(JUSTFNAME(_GENHTML))
? UPPER(JUSTFNAME(_ObjectBrowser)), UPPER(JUSTFNAME(_REPORTBUILDER))
? UPPER(JUSTFNAME(_REPORTOUTPUT)), UPPER(JUSTFNAME(_REPORTPREVIEW))
? UPPER(JUSTFNAME(_SCCTEXT)), UPPER(JUSTFNAME(_TASKLIST)), UPPER(JUSTFNAME(_TASKPANE))
? UPPER(JUSTFNAME(_TOOLBOX)), UPPER(JUSTFNAME(_WIZARD))

* --- the ones that start with nothing in them, because a program is what puts something there
? EMPTY(_GETEXPR), EMPTY(_INCLUDE), EMPTY(_PRETEXT), EMPTY(_SHELL), EMPTY(_SPELLCHK)

* --- the type of each one, which is the part that holds whatever the machine is
* _SAMPLES names the directory the samples were installed to, and _DIARYDATE the day the
* diary opens on, so a golden can say what they are but not what they hold.
? TYPE("_BROWSER"), TYPE("_GALLERY"), TYPE("_SAMPLES"), TYPE("_TOOLBOX")
? TYPE("_CALCMEM"), TYPE("_CALCVALUE"), TYPE("_PAGETOTAL"), TYPE("_DIARYDATE")
? TYPE("_SCREEN"), TYPE("_VFP"), VARTYPE(_SCREEN), VARTYPE(_VFP)

* --- the numbers all start at nothing
? LTRIM(STR(_CALCMEM)), LTRIM(STR(_CALCVALUE)), LTRIM(STR(_PAGETOTAL))

* --- and what a program gets back when it sets one
_PAGETOTAL = 12
_CALCVALUE = 3.5
_CALCMEM = 7
_PRETEXT = "> "
_INCLUDE = "myapp.h"
? LTRIM(STR(_PAGETOTAL)), LTRIM(STR(_CALCVALUE, 10, 1)), LTRIM(STR(_CALCMEM))
? "[" + _PRETEXT + "]", _INCLUDE
_TOOLBOX = "mytoolbox.app"
_SPELLCHK = "myspell.app"
? _TOOLBOX, _SPELLCHK
* a tool variable holds a name; nothing checks that the file behind it is there
? FILE(_TOOLBOX)

* --- a system variable keeps its type, which is what tells it from an ordinary public one
LOCAL oErr
TRY
  _PAGETOTAL = "not a page count"
  ? "a number took a string, and is now " + TYPE("_PAGETOTAL")
CATCH TO oErr
  ? "_PAGETOTAL = string: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  _TOOLBOX = 5
  ? "a name took a number, and is now " + TYPE("_TOOLBOX")
CATCH TO oErr
  ? "_TOOLBOX = number: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
TRY
  _DIARYDATE = "yesterday"
  ? "a date took a string, and is now " + TYPE("_DIARYDATE")
CATCH TO oErr
  ? "_DIARYDATE = string: " + LTRIM(STR(oErr.ErrorNo))
ENDTRY
* the date does take a date, so it is the type and not the name that is fixed
_DIARYDATE = {^2024-07-04}
? DTOC(_DIARYDATE)

* COVERS: _BROWSER, _BUILDER, _CALCMEM, _CALCVALUE, _CODESENSE, _CONVERTER, _DIARYDATE,
* COVERS: _FOXCODE, _FOXREF, _FoxTask, _GALLERY, _GENHTML, _GETEXPR, _INCLUDE,
* COVERS: _ObjectBrowser, _PAGETOTAL, _PRETEXT, _REPORTBUILDER, _REPORTOUTPUT,
* COVERS: _REPORTPREVIEW, _SAMPLES, _SCCTEXT, _SCREEN, _SHELL, _SPELLCHK, _TASKLIST,
* COVERS: _TASKPANE, _TOOLBOX, _VFP, _WIZARD
