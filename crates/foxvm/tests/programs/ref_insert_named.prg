* The table a statement names can be worked out, not only written: this is what the OLE tree
* sample does - it makes a table whose name a dialog chose, then fills it in.
LOCAL lcName
lcName = "tree.dbf"
CREATE TABLE (lcName) (Key c(4), Parent c(4), Text c(60))
INSERT INTO (lcName) VALUES ("1_", "0_", "root")
INSERT INTO (lcName) VALUES ("2_", "1_", "leaf")
USE (lcName)
? RECCOUNT(), ALLTRIM(Key), ALLTRIM(Text)
GO BOTTOM
? ALLTRIM(Parent), ALLTRIM(Text)
* and the alias of a table is the stem of the file it came from, so the whole path finds it
SELECT (lcName)
? ALIAS()

* COVERS: INSERT - SQL
