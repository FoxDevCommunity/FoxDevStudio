* The system variables and the preprocessor, from the language reference.
#DEFINE TITLE "FoxDev"
#DEFINE DEBUG 1
? TITLE
#IFDEF DEBUG
? "debug"
#ELSE
? "quiet"
#ENDIF
#IFNDEF NOTHING
? "no such constant"
#ENDIF
#IF DEBUG
? "on"
#ENDIF
#UNDEF DEBUG
#IFDEF DEBUG
? "still on"
#ELSE
? "off again"
#ENDIF
* the machine, as the platform variables say it is
? _WINDOWS, _DOS, _MAC, _UNIX
* the numbers the language keeps
? _ASCIICOLS, _ASCIIROWS, _DBLCLICK, _INCSEEK
? _PAGENO, _TALLY, _TEXT, _THROTTLE, _TRIGGERLEVEL, _MLINE
* the programs the development environment would run for a job
? _GENMENU, _BEAUTIFY, _COVERAGE
* and the ones a program sets for itself
_CLIPTEXT = "copied"
? _CLIPTEXT
_PAGENO = 7
? _PAGENO
_STARTUP = "boot.prg"
? _STARTUP
* COVERS: #DEFINE ... #UNDEF, #IF ... #ENDIF, #IFDEF, #IFNDEF ... #ENDIF, _ASCIICOLS,
* COVERS: _ASCIIROWS, _BEAUTIFY, _CLIPTEXT, _COVERAGE, _DBLCLICK, _DOS, _GENMENU, _INCSEEK,
* COVERS: _MAC, _MLINE, _PAGENO, _STARTUP, _TALLY, _TEXT, _THROTTLE, _TRIGGERLEVEL, _UNIX,
* COVERS: _WINDOWS
