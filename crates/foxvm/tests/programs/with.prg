WITH oForm
  .Caption = "changed"
  ? .Caption
  WITH .pgfMain.Page1
    .lblGreeting.Caption = "inner"
  ENDWITH
  ? .pgfMain.Page1.lblGreeting.Caption
ENDWITH
? oForm.Caption
? oForm.Parent
? oForm.txtName.Parent.Caption
oForm.txtName.Value = "typed"
? oForm.txtName.Value
? _SCREEN
* COVERS: WITH ... ENDWITH
