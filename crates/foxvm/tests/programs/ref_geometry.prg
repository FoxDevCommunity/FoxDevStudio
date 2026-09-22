* OBJTOCLIENT(): where a control sits on the form it is on, and how big it is.
* CloneObject(): what the product says when a running program asks for it.
LOCAL frm
frm = CREATEOBJECT("Form")
frm.AddObject("cmd", "CommandButton")
frm.cmd.Left = 10
frm.cmd.Top = 20
frm.cmd.Width = 84
frm.cmd.Height = 27

* a control straight on a form is where its own Left and Top say
? OBJTOCLIENT(frm.cmd, 1)
? OBJTOCLIENT(frm.cmd, 2)
* 3 and 4 are the control's own size
? OBJTOCLIENT(frm.cmd, 3)
? OBJTOCLIENT(frm.cmd, 4)

* a control that is on nothing yet is at the corner and still knows how big it is
LOCAL loose
loose = CREATEOBJECT("CommandButton")
? OBJTOCLIENT(loose, 1)
? OBJTOCLIENT(loose, 2)
? OBJTOCLIENT(loose, 3)

* an object with no geometry answers 0 rather than refusing
LOCAL o
o = CREATEOBJECT("Custom")
? OBJTOCLIENT(o, 1)

* which position is asked for runs 1 to 4; anything else is a bad argument, and something
* that is not an object is a type mismatch
TRY
   ? OBJTOCLIENT(frm.cmd, 0)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? OBJTOCLIENT(frm.cmd, 5)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY
TRY
   ? OBJTOCLIENT(1, 1)
CATCH TO oErr
   ? oErr.ErrorNo
ENDTRY

* CloneObject is the designer copying what it is designing, and says so to a running program
TRY
   ? frm.cmd.CloneObject("cmdTwo")
CATCH TO oErr
   ? oErr.ErrorNo
   ? oErr.Message
ENDTRY
? frm.ControlCount

* COVERS: CloneObject, OBJTOCLIENT
