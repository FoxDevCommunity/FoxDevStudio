* COVERS: SET LIBRARY
* Naming the library with an expression in brackets, which is how real code does it: a path is
* worked out - HOME() for the ones the product ships, the application's own folder for its own -
* and never written down. Read as the tail of the line instead, the brackets become part of the
* name and the file is never found.
LOCAL lcName
lcName = "nosuchlibrary.fll"
? FILE(lcName)
TRY
    SET LIBRARY TO (lcName)
CATCH TO oErr
    ? oErr.ErrorNo
ENDTRY
* and with nothing after TO, which lets every library go and names none
SET LIBRARY TO
? SET("LIBRARY") == ""
