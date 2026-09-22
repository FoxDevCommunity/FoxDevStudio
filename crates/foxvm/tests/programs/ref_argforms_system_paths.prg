* COVERS: CURDIR, HOME, VERSION
* HOME(n), CURDIR(cDrive) and VERSION(1) each answer with a path or a build that differs
* between machines, so a golden cannot assert the value itself - only its shape, the way
* ref_machine.prg already does for the clock and the keyboard.
? "HOME(1) is a nonempty path", LEN(HOME(1)) > 0
? "HOME(2) is a nonempty path", LEN(HOME(2)) > 0
? "CURDIR(cDrive) is a string", VARTYPE(CURDIR("C")) == "C"

* VERSION(1) is not the same text as the plain form - measured against the product, it is the
* longer one, with a build date and a product ID folded in - so only its shape is asked here.
? "VERSION(1) is a nonempty string", LEN(VERSION(1)) > 0
