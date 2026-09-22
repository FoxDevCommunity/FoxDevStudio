* Building: what a program that ships another program asks the IDE for. The project is made
* out of the files it names, then turned into the file that ships; COMPILE checks source
* without running it.
BUILD PROJECT ship FROM main.prg, entry.fxf
BUILD APP ship.app FROM ship RECOMPILE
BUILD EXE ship.exe FROM ship
COMPILE *.prg
COMPILE FORM entry
? "built a project, an app and an executable, and compiled twice"

* COVERS: BUILD APP, BUILD EXE, BUILD PROJECT, COMPILE
