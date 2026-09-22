ON ERROR ?? "[ERR " + LTRIM(STR(ERROR())) + " " + MESSAGE() + "]"
SET ALTERNATE TO edge2.txt
SET ALTERNATE ON
LOCAL i, s
? "fdow 8 "
SET FDOW TO 8
? "point empty "
SET POINT TO ""
? "nulldisplay 38 "
SET NULLDISPLAY TO "a very long null display string indeed"
? "nulldisplay 24 "
SET NULLDISPLAY TO "123456789012345678901234"
? "  got [" + SET("NULLDISPLAY") + "]"
? "nulldisplay 25 "
SET NULLDISPLAY TO "1234567890123456789012345"
? "  got [" + SET("NULLDISPLAY") + "]"
SET NULLDISPLAY TO
? "date bad "
SET DATE TO NOSUCH
? "decimals 19 "
SET DECIMALS TO 19
? "hours 13 "
SET HOURS TO 13
? "sep empty "
SET SEPARATOR TO ""
? "mark empty ["
SET MARK TO ""
?? SET("MARK") + "]"
? "currency empty ["
SET CURRENCY TO ""
?? SET("CURRENCY",1) + "]"
SET CURRENCY TO
SET ALTERNATE OFF
SET ALTERNATE TO
QUIT
