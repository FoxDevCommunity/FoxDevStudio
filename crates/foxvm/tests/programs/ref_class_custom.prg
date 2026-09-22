* The objects a program makes for itself: Custom, Session, Collection, Hyperlink and Empty.
LOCAL oCustom, oSession, oItems, oLink, oNothing
oCustom = CREATEOBJECT("Custom")
? oCustom.BaseClass, oCustom.Class, oCustom.Name, "[" + oCustom.ParentClass + "]"
? TRANSFORM(oCustom.Width), TRANSFORM(oCustom.Height), TRANSFORM(oCustom.Left), TRANSFORM(oCustom.Top)
? TRANSFORM(oCustom.ControlCount), "[" + oCustom.Picture + "]", "[" + oCustom.Tag + "]"
? PEMSTATUS(oCustom, "AddObject", 5), PEMSTATUS(oCustom, "AddProperty", 5), PEMSTATUS(oCustom, "Init", 5)
? PEMSTATUS(oCustom, "Visible", 5), PEMSTATUS(oCustom, "Click", 5), PEMSTATUS(oCustom, "Enabled", 5)

* a property a program adds is one the object then answers to
? ADDPROPERTY(oCustom, "Total", 0)
oCustom.Total = 42
? TRANSFORM(oCustom.Total), PEMSTATUS(oCustom, "Total", 5)
? REMOVEPROPERTY(oCustom, "Total"), PEMSTATUS(oCustom, "Total", 5)

* a session is a data session of its own, and nothing else
oSession = CREATEOBJECT("Session")
? oSession.BaseClass, oSession.Class, oSession.Name, TRANSFORM(oSession.DataSession)
? PEMSTATUS(oSession, "DataSessionID", 5), PEMSTATUS(oSession, "AddObject", 5), PEMSTATUS(oSession, "Init", 5)

* a collection holds things by key, and starts out empty
oItems = CREATEOBJECT("Collection")
? oItems.BaseClass, oItems.Class, oItems.Name, TRANSFORM(oItems.Count), TRANSFORM(oItems.KeySort)
oItems.KeySort = 1
? TRANSFORM(oItems.KeySort)
? PEMSTATUS(oItems, "Add", 5), PEMSTATUS(oItems, "Remove", 5), PEMSTATUS(oItems, "Item", 5), PEMSTATUS(oItems, "GetKey", 5)

* a hyperlink is the thing that opens a page, and holds nothing
oLink = CREATEOBJECT("Hyperlink")
? oLink.BaseClass, oLink.Class, oLink.Name
? PEMSTATUS(oLink, "NavigateTo", 5), PEMSTATUS(oLink, "GoBack", 5), PEMSTATUS(oLink, "GoForward", 5)

* an Empty has nothing at all until a program puts something on it
oNothing = CREATEOBJECT("Empty")
? VARTYPE(oNothing), PEMSTATUS(oNothing, "Name", 5), PEMSTATUS(oNothing, "Init", 5)
? ADDPROPERTY(oNothing, "Colour", "red"), oNothing.Colour, PEMSTATUS(oNothing, "Colour", 5)
* COVERS: Custom, Session, Collection, Hyperlink, Empty, KeySort, Count, GetKey, DataSessionID
