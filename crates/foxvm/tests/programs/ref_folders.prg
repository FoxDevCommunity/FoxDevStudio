* Making, entering and removing folders, and where a relative name is taken from. Nothing here
* prints a path: the folder a program runs in is a different one every time it is measured.
SET SAFETY OFF
MKDIR shed
? DIRECTORY("shed")
CD shed
? TRANSFORM(STRTOFILE("inside", "kept.txt"))
? FILE("kept.txt")
CD ..
? FILE("kept.txt"), FILE("shed\kept.txt")
CHDIR shed
? FILE("kept.txt")
CHDIR ..

* SET DEFAULT says where a relative name is taken from without moving the program
MD barn
SET DEFAULT TO barn
? TRANSFORM(STRTOFILE("stored", "hay.txt"))
? FILE("hay.txt")
SET DEFAULT TO ..
? FILE("hay.txt"), FILE("barn\hay.txt")

* and the files and the folders go again
DELETE FILE shed\kept.txt
DELETE FILE barn\hay.txt
? FILE("shed\kept.txt"), FILE("barn\hay.txt")
RMDIR shed
RD barn
? DIRECTORY("shed"), DIRECTORY("barn")
* COVERS: CD, CHDIR, DELETE FILE, DIRECTORY, MKDIR, RMDIR, SET DEFAULT
