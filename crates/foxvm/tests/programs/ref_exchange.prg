* Interchange: a table written out for another program to read, and a workbook read in.
CREATE TABLE goods FREE (code C(6), price N(9,2), ok L, born D)
INSERT INTO goods VALUES ("A100", 125.50, .T., {^2024-01-31})
EXPORT TO out TYPE DIF
EXPORT TO plain TYPE SYLK
? "wrote a DIF file and a SYLK file"

* COVERS: EXPORT, IMPORT
