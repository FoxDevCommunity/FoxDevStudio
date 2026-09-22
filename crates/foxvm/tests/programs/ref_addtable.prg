* COVERS: ADD TABLE, REMOVE TABLE, FREE TABLE
* A table belongs to one database at a time. CREATE TABLE puts the table it makes into whatever
* database is open, so adding it again is not a second membership but an error - the product
* answers 1537, "Cannot add this table: it belongs to database <name>". Only after it has been
* taken out again can it be added.
CREATE DATABASE shop
CREATE TABLE stock (code C(4), name C(12))
USE
? INDBC("stock", "TABLE")
TRY
    ADD TABLE stock
CATCH TO oErr
    ? oErr.ErrorNo
ENDTRY
REMOVE TABLE stock
? INDBC("stock", "TABLE")
ADD TABLE stock
? INDBC("stock", "TABLE")
CLOSE DATABASES
