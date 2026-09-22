* The font functions' argument forms a golden had not yet called: AFONT asked about one font
* by name rather than filling the array with every one, FONTMETRIC asked without a font or with
* one but no size, and WFONT asked about a font by window name.
*
* AFONT(array) alone already has a golden (ref_array.prg): it fills the array with every font
* and answers the count. Naming a font instead asks something else about that one font, and
* answers whether the array was made rather than a count - what the array itself holds is a
* real font's real metrics, which is a different shape on every machine, so what is measured is
* the type of each answer rather than its value.
LOCAL ARRAY aFontSizes(1)
? AFONT(aFontSizes, "Arial")
? VARTYPE(aFontSizes(1))
? AFONT(aFontSizes, "Arial", 10)
? VARTYPE(aFontSizes(1))

* FONTMETRIC(nAttribute) alone measures the active output window's own font; naming a font
* without its size is half of a pair the reference asks for together.
? VARTYPE(FONTMETRIC(1)) == "N" AND FONTMETRIC(1) > 0
TRY
  ? FONTMETRIC(1, "Arial")
CATCH TO oErr
  ? "caught", TRANSFORM(oErr.ErrorNo)
ENDTRY
? VARTYPE(FONTMETRIC(1, "Arial", 10, "B")) == "N" AND FONTMETRIC(1, "Arial", 10, "B") > 0

* WFONT(nAttribute, cWindow): the same three attributes, asked of a window by name rather than
* of the active output window - the font itself is the shell's own and differs by machine, so
* only its type is measured.
? VARTYPE(WFONT(1, "")) == "C"
? VARTYPE(WFONT(2, "")) == "N" AND WFONT(2, "") > 0
? WFONT(3, "")

* COVERS: AFONT, FONTMETRIC, WFONT
