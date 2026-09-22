* Writes fdvclasses.vcx, the class library the tests read.
*
* A `.vcx` is a table with a memo file beside it, so one can be written without the Class
* Designer - which waits for a person and so can be asked nothing in a test run. Run this with
* `vfp9.exe fdvclasses.gen.prg` in a scratch folder and copy fdvclasses.vcx and fdvclasses.vct
* into `golden` beside it. What the run writes into out.txt is the product reading its own file
* back, which is what says the result is a class library and not merely a table.
*
* Three things the product insists on, each measured by leaving it out and watching it refuse:
* the first record stamps the file (COMMENT / Class / VERSION = 3.00) or the file is "invalid";
* RESERVED2 on a class row counts the records the class spans, or its members are not read at
* all; and RESERVED3 lists the members the class adds of its own, or a property written in the
* memo is "not found" on the object. A custom character property is written unquoted, which is
* how the designer writes one - quoted, the quotes come back as part of the value.
PUBLIC gcNL
gcNL = CHR(13) + CHR(10)
LOCAL lcOut

CREATE TABLE fdvclasses.dbf FREE (PLATFORM C(8), UNIQUEID C(10), TIMESTAMP N(10), ;
  CLASS M, CLASSLOC M, BASECLASS M, OBJNAME M, PARENT M, PROPERTIES M, PROTECTED M, ;
  METHODS M, OBJCODE M, OLE M, OLE2 M, RESERVED1 M, RESERVED2 M, RESERVED3 M, ;
  RESERVED4 M, RESERVED5 M, RESERVED6 M, RESERVED7 M, RESERVED8 M, USER M)

* the stamp that makes the table a class library rather than a table
INSERT INTO fdvclasses (PLATFORM, UNIQUEID, TIMESTAMP, RESERVED1) VALUES ("COMMENT ", "Class     ", 0, "VERSION =   3.00")

* fdvgreeter: properties of its own, a method, and an Init that runs
DO fdvClass WITH "fdvgreeter", "custom", "custom", "", 1, ;
  'cGreeting = hello' + gcNL + 'nCount = 2' + gcNL + 'Name = "fdvgreeter"' + gcNL, ;
  'PROCEDURE Init' + gcNL + 'THIS.cGreeting = "hello from the library"' + gcNL + 'ENDPROC' + gcNL + ;
  'PROCEDURE Greet' + gcNL + 'RETURN THIS.cGreeting + " (" + TRANSFORM(THIS.nCount) + ")"' + gcNL + 'ENDPROC' + gcNL, ;
  "cgreeting" + gcNL + "ncount" + gcNL + "*greet" + gcNL

* fdvchild: a class of this library built on another one in it
DO fdvClass WITH "fdvchild", "fdvgreeter", "custom", "fdvclasses.vcx", 1, ;
  'nCount = 5' + gcNL + 'Name = "fdvchild"' + gcNL, ;
  'PROCEDURE Shout' + gcNL + 'RETURN UPPER(THIS.Greet())' + gcNL + 'ENDPROC' + gcNL, ;
  "*shout" + gcNL

* fdvpanel: a container with a button inside it, so a class with members can be made
DO fdvClass WITH "fdvpanel", "container", "container", "", 2, ;
  'Width = 120' + gcNL + 'Height = 40' + gcNL + 'cPanel = panel' + gcNL + 'Name = "fdvpanel"' + gcNL, ;
  'PROCEDURE Init' + gcNL + 'THIS.cmdgo.Caption = "ready"' + gcNL + 'ENDPROC' + gcNL, ;
  "cpanel" + gcNL
INSERT INTO fdvclasses (PLATFORM, UNIQUEID, TIMESTAMP, CLASS, CLASSLOC, BASECLASS, OBJNAME, PARENT, PROPERTIES) ;
  VALUES ("WINDOWS ", "_FDVMEMBER", 0, "commandbutton", "", "commandbutton", "cmdgo", "fdvpanel", ;
  'Caption = "Go"' + gcNL + 'Left = 5' + gcNL + 'Top = 5' + gcNL + 'Name = "cmdgo"' + gcNL)
INSERT INTO fdvclasses (PLATFORM, UNIQUEID, TIMESTAMP, OBJNAME) VALUES ("COMMENT ", "RESERVED  ", 0, "fdvpanel")

* fdvbutton: a control class, for a container that makes one of its own
DO fdvClass WITH "fdvbutton", "commandbutton", "commandbutton", "", 1, ;
  'Caption = "Library button"' + gcNL + 'Width = 90' + gcNL + 'cTag = from the library' + gcNL + 'Name = "fdvbutton"' + gcNL, ;
  'PROCEDURE Click' + gcNL + 'THIS.Caption = "clicked"' + gcNL + 'ENDPROC' + gcNL, ;
  "ctag" + gcNL

USE
RENAME fdvclasses.dbf TO fdvclasses.vcx
RENAME fdvclasses.fpt TO fdvclasses.vct
COMPILE CLASSLIB fdvclasses.vcx

* and read it back, so the file is known to work before it is kept
SET CLASSLIB TO fdvclasses
lcOut = "classlib=[" + JUSTFNAME(STREXTRACT(SET("CLASSLIB"), '"', '"')) + "]" + gcNL
o = CREATEOBJECT("fdvgreeter")
lcOut = lcOut + "greeter: " + o.Greet() + " class=" + o.Class + " base=" + o.BaseClass + gcNL
o = CREATEOBJECT("fdvchild")
lcOut = lcOut + "child: " + o.Shout() + " class=" + o.Class + " parentclass=" + o.ParentClass + gcNL
o = CREATEOBJECT("fdvpanel")
lcOut = lcOut + "panel: controls=" + TRANSFORM(o.ControlCount) + " button=[" + o.cmdgo.Caption + "] panel=" + o.cPanel + gcNL
o = CREATEOBJECT("fdvbutton")
lcOut = lcOut + "button: [" + o.Caption + "] tag=[" + o.cTag + "] base=" + o.BaseClass + gcNL
SET CLASSLIB TO
STRTOFILE(lcOut, "out.txt")

PROCEDURE fdvClass
LPARAMETERS cName, cClass, cBase, cLoc, nRows, cProps, cMethods, cMembers
INSERT INTO fdvclasses (PLATFORM, UNIQUEID, TIMESTAMP, CLASS, CLASSLOC, BASECLASS, OBJNAME, PARENT, PROPERTIES, METHODS, RESERVED1, RESERVED2, RESERVED3) ;
  VALUES ("WINDOWS ", "_FDV" + PADL(RECCOUNT() + 1, 6, "0"), 0, cClass, cLoc, cBase, cName, "", cProps, cMethods, "Class", TRANSFORM(nRows), cMembers)
ENDPROC
