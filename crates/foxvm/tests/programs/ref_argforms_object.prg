* COVERS: CREATEOBJECTEX
* CREATEOBJECTEX takes a class name and a computer name, and the reference page gives that
* second one with no brackets around it at all - so the class name alone is not "the computer
* defaulted"; it is error 1229, "Too few arguments", the same shape of gap DISPLAYPATH() has.
* A third, optional argument names an interface. Calling with either the computer or the
* interface argument present needs a live COM object to ask anything of, which this runtime
* reaches through the `foxole` napi addon - not something the VM's own golden tests have - so
* those forms stay on tests/vfp/arg-forms-known.txt rather than being asked here.
LOCAL oErr, cSaid
TRY
  cSaid = "unreached"
  = CREATEOBJECTEX("Scripting.FileSystemObject")
CATCH TO oErr
  cSaid = "error " + LTRIM(STR(oErr.ErrorNo)) + " " + oErr.Message
ENDTRY
? "CREATEOBJECTEX(1)", cSaid
