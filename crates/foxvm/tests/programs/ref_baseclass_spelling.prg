* How Visual FoxPro spells a class name back: first letter up, the rest down, whatever the
* program wrote. BaseClass follows the same rule as Class, so a control built from a base class
* and one built from a class of its own answer in the same shape - which matters because $ and =
* are case-sensitive, and plenty of sample code compares BaseClass to a literal it typed in
* camel case.
LOCAL o, oSub, oOne

o = CREATEOBJECT("CommandButton")
? o.BaseClass, o.Class, "[" + o.ParentClass + "]"
oOne = CREATEOBJECT("PageFrame")
? oOne.BaseClass
oOne = CREATEOBJECT("OptionGroup")
? oOne.BaseClass
oOne = CREATEOBJECT("TextBox")
? oOne.BaseClass
oOne = CREATEOBJECT("EditBox")
? oOne.BaseClass
oOne = CREATEOBJECT("XMLAdapter")
? oOne.BaseClass

* the spelling a comparison sees
? o.BaseClass == "Commandbutton", o.BaseClass == "CommandButton"
? o.BaseClass = "CommandButton", "button" $ o.BaseClass, "button" $ LOWER(o.BaseClass)

* a class of the program's own: the base class underneath it is spelled the same way, and so is
* the name the program gave the class
oSub = CREATEOBJECT("mybutton")
? oSub.BaseClass, oSub.Class

DEFINE CLASS mybutton AS CommandButton
	Caption = "Press"
ENDDEFINE
