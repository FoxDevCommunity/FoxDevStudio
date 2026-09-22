* COVERS: CPCURRENT, IMESTATUS
* CPCURRENT([1 | 2]): the plain form and `1` both answer the code page Windows uses for text;
* `2` measures as the OEM one a DOS box uses instead, regardless of the CODEPAGE setting.
? "CPCURRENT() is numeric", VARTYPE(CPCURRENT()) == "N"
? "CPCURRENT(1) agrees with the plain form", CPCURRENT(1) == CPCURRENT()
? "CPCURRENT(2) is the OEM code page instead", CPCURRENT(2) == 437

* IMESTATUS([n]): on a machine with no Input Method Editor, turning it off changes nothing to
* measure, so what is left is that the call is accepted and still answers 0.
? "IMESTATUS() is 0 with no IME", IMESTATUS() == 0
? "IMESTATUS(0) still answers 0", IMESTATUS(0) == 0
