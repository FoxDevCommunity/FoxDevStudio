* What a program is holding, written out. The ones that list a database's tables and views are
* not here: Visual FoxPro answers those with full paths, and the folder a program runs in is a
* different one every time it is measured.
*
* The names run in alphabetical order on purpose. Visual FoxPro lists the variables in the
* order they were made and this runtime lists them by name, so a program that made them in any
* other order would be watching that difference rather than the layout, which is the point here.
PUBLIC zzA, zzB, zzC, zzD
zzA = "kept"
zzB = 7
zzC = .T.
zzD = {^2004-01-02}
DIMENSION zzE[2]
zzE[1] = "one"
zzE[2] = "two"
DISPLAY MEMORY LIKE zz*
? "[shown]"
RELEASE zzA, zzB, zzC, zzD, zzE
DISPLAY MEMORY LIKE zz*
? "[nothing left]"
* COVERS: DISPLAY MEMORY
