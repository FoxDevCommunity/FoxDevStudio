* The index functions asked about a tag, and about a table in a work area that is not the
* current one - the two arguments together, which is the form nobody had measured.
ON ERROR ?? ""
SET SAFETY OFF

CREATE TABLE staff (name C(10), dept C(6), pay N(8,2))
INSERT INTO staff (name, dept, pay) VALUES ("Nolan", "SALES", 3200)
INSERT INTO staff (name, dept, pay) VALUES ("Ames", "ADMIN", 4100)
INSERT INTO staff (name, dept, pay) VALUES ("Curtis", "SALES", 2750)
INDEX ON name TAG byname
INDEX ON dept TAG bydept UNIQUE
INDEX ON pay TAG bypay DESCENDING CANDIDATE

SELECT 2
CREATE TABLE dept (code C(4), title C(10))
INSERT INTO dept (code, title) VALUES ("S1", "Sales")
INSERT INTO dept (code, title) VALUES ("A1", "Admin")
INDEX ON code TAG bycode CANDIDATE

SELECT 1

* --- CDX(), NDX(), MDX(): asked about a table in another work area, by number and by alias
? UPPER(JUSTFNAME(CDX(1, 2)))
? UPPER(JUSTFNAME(CDX(1, "dept")))
? NDX(1, 2)
? UPPER(JUSTFNAME(MDX(1, 2)))

* --- IDXCOLLATE(): a tag in another work area, and named by its compound index file too
? IDXCOLLATE(1, 2)
? IDXCOLLATE(1, "dept")
? IDXCOLLATE("DEPT", 1, 2)

* --- TAG(): asked about a table elsewhere, and named by its compound index file too
? TAG(1, 2)
? TAG(1, "dept")
? TAG("DEPT", 1, 2)

* --- TAGCOUNT(): a compound index file name, and one in another work area
? TAGCOUNT("STAFF")
? TAGCOUNT("", 2)
? TAGCOUNT("DEPT", "dept")

* --- TAGNO(): a tag with the compound index named, and one in another work area
? TAGNO("bydept", "STAFF")
? TAGNO("bycode", "DEPT", 2)
? TAGNO("bycode", "", "dept")

* --- ORDER(): asked about another work area, and with the path flag
? ORDER(2)
? ORDER("dept")
? UPPER(JUSTFNAME(ORDER(2, 1)))
? UPPER(JUSTFNAME(ORDER("dept", 1)))

* --- KEY(): a tag in another work area
? KEY(1, 2)
? KEY(1, "dept")

* --- DESCENDING(): a tag in another work area
? DESCENDING(1, 2)
? DESCENDING(1, "dept")
? DESCENDING(3, 1)

* --- UNIQUE(): a tag in another work area
? UNIQUE(2, 1)
? UNIQUE(1, 2)
? UNIQUE(1, "dept")

* --- CANDIDATE(): a tag in another work area
? CANDIDATE(3, 1)
? CANDIDATE(1, 2)
? CANDIDATE(1, "dept")

* --- PRIMARY(): a tag in another work area - no databases here, so never true
? PRIMARY(1, 2)
? PRIMARY(1, "dept")

* --- ATAGINFO(): named .cdx file, and one in another work area
LOCAL aTags(1), aTags2(1)
? ATAGINFO(aTags, "STAFF")
? aTags(1, 1), aTags(2, 1), aTags(3, 1)
? ATAGINFO(aTags2, "DEPT", 2)
? aTags2(1, 1)

SELECT 2
USE
SELECT 1
USE

* COVERS: ATAGINFO, CANDIDATE, CDX, DESCENDING, IDXCOLLATE, KEY, MDX, NDX, ORDER, PRIMARY, TAG, TAGCOUNT, TAGNO, UNIQUE
