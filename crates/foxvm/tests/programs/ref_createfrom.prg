* A table made from a table that describes one: COPY STRUCTURE EXTENDED writes the description,
* CREATE ... FROM reads it back and makes the table it describes.
CREATE TABLE goods FREE (code C(6), price N(9,2))
COPY STRUCTURE EXTENDED TO shape
USE shape
? RECCOUNT(), FCOUNT()
? ALLTRIM(field_name), field_type, field_len, field_dec
REPLACE field_len WITH 12, field_dec WITH 4 FOR ALLTRIM(field_name) == "PRICE"
USE
CREATE spare FROM shape
? ALIAS(), RECCOUNT(), FCOUNT()
? FIELD(2), TYPE("price")

* COVERS: CREATE FROM
