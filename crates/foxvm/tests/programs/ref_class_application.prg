* The application object: `_VFP`, and what every object's Application property answers with.
*
* There is one of it and a program never makes one, so what it holds is read rather than set.
* The things that are about this copy rather than about the class - the process it is running as,
* the directory it was started in, the file it was started from - are asked about by name and not
* printed, because no two runs would agree about them.
LOCAL oCustom
? _VFP.Name
? _VFP.Caption
? _VFP.Version
? TRANSFORM(_VFP.StartMode), TRANSFORM(_VFP.LanguageOptions)
? TRANSFORM(_VFP.OLERequestPendingTimeout), TRANSFORM(_VFP.OLEServerBusyTimeout), _VFP.OLEServerBusyRaiseError
? "[" + _VFP.VFPXMLProgID + "]"

* the ones whose value is of this run: that they are there is the whole of what can be said
? PEMSTATUS(_VFP, "ProcessId", 5), PEMSTATUS(_VFP, "ThreadId", 5), PEMSTATUS(_VFP, "hWnd", 5)
? PEMSTATUS(_VFP, "FullName", 5), PEMSTATUS(_VFP, "ServerName", 5), PEMSTATUS(_VFP, "DefaultFilePath", 5)
? PEMSTATUS(_VFP, "AutoYield", 5), PEMSTATUS(_VFP, "EditorOptions", 5), PEMSTATUS(_VFP, "StatusBar", 5)
? PEMSTATUS(_VFP, "ActiveForm", 5), PEMSTATUS(_VFP, "ActiveProject", 5), PEMSTATUS(_VFP, "Visible", 5)
? PEMSTATUS(_VFP, "Forms", 5), PEMSTATUS(_VFP, "Projects", 5), PEMSTATUS(_VFP, "Objects", 5)

* what it can be asked to do. None of them is called: Quit would end the program that asked.
? PEMSTATUS(_VFP, "DoCmd", 5), PEMSTATUS(_VFP, "Eval", 5), PEMSTATUS(_VFP, "SetVar", 5)
? PEMSTATUS(_VFP, "Quit", 5), PEMSTATUS(_VFP, "Help", 5), PEMSTATUS(_VFP, "DataToClip", 5)
? PEMSTATUS(_VFP, "RequestData", 5)

* every object belongs to the same application, and says so
oCustom = CREATEOBJECT("Custom")
? VARTYPE(oCustom.Application), oCustom.Application.Name == _VFP.Name
* COVERS: Application, AutoYield, DefaultFilePath, EditorOptions, Forms, FullName, hWnd,
* COVERS: LanguageOptions, OLERequestPendingTimeout, OLEServerBusyRaiseError,
* COVERS: OLEServerBusyTimeout, PROCESSID, ServerName, StartMode, StatusBar, ThreadID,
* COVERS: VFPXMLProgID, ActiveForm, ActiveProject, DataToClip, Eval, RequestData, SetVar
* the application has a Help and a Quit of its own, and the reference has a HELP and a QUIT
* command as well; naming the kind keeps the claim off the two commands, which open a window and
* end the session and so are not exercised here
* COVERS: method:Help, method:Quit
