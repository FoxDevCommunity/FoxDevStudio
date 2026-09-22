LOCAL a(3)
a[2] = 5
? a[1], a[2], a(2)
DIMENSION b(2, 2)
b(2, 1) = "x"
b[1, 2] = 7
? b(2, 1), b[1, 2], b[3]
? b[4]
LOCAL c(2)
c[1] = "first"
DIMENSION c(4)
? c[1], c[4]
DO Fill WITH @a
? a[3]
STORE 0 TO t1, t2
t1 = 1
? t1, t2
STORE "s" TO a[1], b[1, 1]
? a[1], b[1, 1]
PRIVATE p(2)
p[2] = "priv"
DO ShowP
? a[4]

PROCEDURE Fill(arr)
  arr[3] = "filled"

PROCEDURE ShowP
  ? p(2)
* COVERS: DIMENSION, DO, LOCAL, PRIVATE, PROCEDURE, STORE
