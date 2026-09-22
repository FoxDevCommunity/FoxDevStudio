* The formset base class, and what THIS, THISFORM and THISFORMSET say outside a method.
*
* A formset is a container whose members are whole forms shown together; THISFORMSET is how
* anything inside one reaches it. The three keywords each name themselves when they are used
* where no method is running, which is one error code and three different messages.
*
* The value is worked out into a variable before it is printed: `?` moves to a new line before
* it evaluates anything, so printing an expression that fails leaves a blank line behind.
LOCAL oSet, oErr, uAnswer
oSet = CREATEOBJECT("Formset")
? oSet.BaseClass, oSet.Class, oSet.Name
? VARTYPE(oSet)
* nothing has been added to it, so it holds no forms and is visible all the same
? TRANSFORM(oSet.FormCount)
? oSet.Visible
* it goes away of its own accord only if it is told to
? oSet.AutoRelease, TRANSFORM(oSet.WindowType), "[" + oSet.WindowList + "]"
* the READ settings a converted FoxPro 2.x screen set was written with live on the set
? oSet.ReadCycle, oSet.ReadLock, oSet.ReadMouse, oSet.ReadSave, TRANSFORM(oSet.ReadTimeout)
? PEMSTATUS(oSet, "ReadActivate", 5), PEMSTATUS(oSet, "ReadDeactivate", 5), PEMSTATUS(oSet, "ReadShow", 5)
? PEMSTATUS(oSet, "ReadValid", 5), PEMSTATUS(oSet, "ReadWhen", 5), PEMSTATUS(oSet, "ActiveForm", 5)
? PEMSTATUS(oSet, "AddObject", 5), PEMSTATUS(oSet, "Release", 5), PEMSTATUS(oSet, "Forms", 5)
TRY
	uAnswer = THISFORMSET.Name
	? uAnswer
CATCH TO oErr
	? TRANSFORM(oErr.ErrorNo), oErr.Message
ENDTRY
TRY
	uAnswer = THISFORM.Name
	? uAnswer
CATCH TO oErr
	? TRANSFORM(oErr.ErrorNo), oErr.Message
ENDTRY
TRY
	uAnswer = THIS.Name
	? uAnswer
CATCH TO oErr
	? TRANSFORM(oErr.ErrorNo), oErr.Message
ENDTRY
* COVERS: BaseClass, Class, CREATEOBJECT, FormCount, FormSet, Name, TRANSFORM, TRY, VARTYPE,
* COVERS: Visible, AutoRelease, WindowList, ReadCycle, ReadLock, ReadMouse, ReadSave, ReadTimeout
