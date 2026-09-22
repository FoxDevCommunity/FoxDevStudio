* `\` and `\\` send a line to the text merge output; SET TEXTMERGE TO says where that goes.
PRIVATE cOut
LOCAL cLocal, cBlock, cName
cName = "Fox"
SET TEXTMERGE TO MEMVAR cOut NOSHOW
SET TEXTMERGE ON
\first
\\ and more on the same line
\  the spaces in front are kept
\
\after an empty line
\<<cName>> at the front
\", "
SET TEXTMERGE TO
? cOut

* SET TEXTMERGE OFF leaves what stands between the delimiters where it is
SET TEXTMERGE OFF
SET TEXTMERGE TO MEMVAR cOut NOSHOW
\plain <<cName>>
SET TEXTMERGE TO
SET TEXTMERGE ON
? cOut

* the delimiters are whatever SET TEXTMERGE DELIMITERS last named
SET TEXTMERGE DELIMITERS TO "{{", "}}"
SET TEXTMERGE TO MEMVAR cOut NOSHOW
\<<cName>> stays and {{cName}} is worked out
SET TEXTMERGE TO
? cOut
? SET("TEXTMERGE", 1)
SET TEXTMERGE DELIMITERS TO
? SET("TEXTMERGE", 1)

* a LOCAL is found by its name, and ADDITIVE builds on what the variable holds
cLocal = "seed"
SET TEXTMERGE TO MEMVAR cLocal ADDITIVE NOSHOW
\one
\two
SET TEXTMERGE TO
? cLocal

* a file has a line in progress the moment it is opened, so the first `\` ends it
SET TEXTMERGE TO merged.txt NOSHOW
\alpha
\\beta
SET TEXTMERGE TO
? "[" + FILETOSTR("merged.txt") + "]"

* nothing inside a TEXT block is a command, a comment or a continuation
TEXT TO cBlock NOSHOW TEXTMERGE
name=<<cName>>
* not a comment && nor is this
one; two
ENDTEXT and the rest of the line is ignored
? cBlock

* SET TEXTMERGE ON is enough for a block that does not say TEXTMERGE itself
TEXT TO cBlock NOSHOW
still <<cName>>
ENDTEXT
? cBlock
* COVERS: \, \\, SET TEXTMERGE, SET TEXTMERGE DELIMITERS, TEXT ... ENDTEXT
