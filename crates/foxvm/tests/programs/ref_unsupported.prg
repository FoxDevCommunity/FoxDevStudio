* A command this runtime cannot honour still compiles.
*
* Visual FoxPro compiles `BUILD DLL` without a murmur - written out, run through COMPILE, and no
* .err file appears beside the .fxp - and fails only if the line is reached. Refusing it at
* compile time took the whole method with it, so every other line in the method stopped working
* too. What each product then says is its own: VFP goes looking for the project and cannot find
* it, this runtime says the feature is not here. That the line is reached at all is the point.
? "before"
TRY
   BUILD DLL server FROM server RECOMPILE
CATCH
   ? "reached, and refused"
ENDTRY
? "after"
* COVERS: BUILD DLL
