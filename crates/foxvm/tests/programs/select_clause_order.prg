* SELECT's clauses after the columns come in any order
LOCAL aOut[1]
CREATE CURSOR reg (name C(10), type C(5))
INSERT INTO reg VALUES ("beta", "A")
INSERT INTO reg VALUES ("alpha", "A")
INSERT INTO reg VALUES ("beta", "B")
INSERT INTO reg VALUES ("gamma", "A")
SELECT name, COUNT(*) AS n ;
  FROM reg ;
  WHERE type = "A" OR type = "B" ;
  INTO ARRAY aOut ;
  ORDER BY name ;
  GROUP BY name
? _TALLY, ALEN(aOut, 1)
? aOut[1,1], aOut[1,2], aOut[2,1], aOut[2,2], aOut[3,1], aOut[3,2]
SELECT name FROM reg INTO CURSOR c1 WHERE type = "B"
? _TALLY, c1.name
SELECT name FROM reg ORDER BY name DESC WHERE type = "A" INTO CURSOR c2
? _TALLY, c2.name
