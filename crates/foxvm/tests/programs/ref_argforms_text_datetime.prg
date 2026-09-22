* DATE()/DATETIME() as constructors: nYear, nMonth and nDay each have a hard range
* (100-9999, 1-12, 1-31) that error 11 outside it. A day that passes those bounds but does not
* exist in that particular month - DATE(2024, 2, 30) - is not an error at all: it is the empty
* date, not a range check this runtime used to fold into the same error as an impossible month.
SET DATE TO AMERICAN
SET CENTURY ON
? "DATE an ordinary one", DATE(2024, 3, 15)
? "DATE the earliest year the range allows", DATE(100, 1, 1)
? "DATE February 30th is the empty date", DATE(2024, 2, 30)
? "DATE February 29th in a leap year", DATE(2024, 2, 29)
TRY
	? DATE(2024, 13, 1)
CATCH TO oErr
	? "DATE month 13 is out of the structural range", oErr.ErrorNo
ENDTRY
TRY
	? DATE(99, 1, 1)
CATCH TO oErr
	? "DATE year 99 is below the range", oErr.ErrorNo
ENDTRY
LOCAL ldGot
TRY
	ldGot = DATE(2024)
CATCH TO oErr
	? "DATE with only a year is too few arguments, not error 11", oErr.ErrorNo
ENDTRY
TRY
	ldGot = DATE(2024, 3)
CATCH TO oErr
	? "DATE with a year and a month is still too few", oErr.ErrorNo
ENDTRY

? "DATETIME with hours, minutes and seconds", DATETIME(2024, 3, 15, 13, 45, 30)
? "DATETIME with only an hour", DATETIME(2024, 3, 15, 5)
? "DATETIME with an hour and a minute", DATETIME(2024, 3, 15, 5, 30)
? "DATETIME with none of hms is midnight", DATETIME(2024, 3, 15)
TRY
	? DATETIME(2024, 3, 15, 24, 0, 0)
CATCH TO oErr
	? "DATETIME hour 24 is out of range", oErr.ErrorNo
ENDTRY
TRY
	ldGot = DATETIME(2024)
CATCH TO oErr
	? "DATETIME with only a year is too few arguments", oErr.ErrorNo
ENDTRY
TRY
	ldGot = DATETIME(2024, 3)
CATCH TO oErr
	? "DATETIME with a year and a month is still too few", oErr.ErrorNo
ENDTRY

* DTOC(): a second argument switches to the unformatted YYYYMMDD form by being there at all,
* not by what value it holds - measured, DTOC(d, 0) is the same as DTOC(d, 1).
LOCAL ldD
ldD = DATE(2024, 3, 15)
? "DTOC formatted, no second argument", DTOC(ldD)
? "DTOC with a 1", DTOC(ldD, 1)
? "DTOC with a 0 is the same unformatted form", DTOC(ldD, 0)

* QUARTER(): the reference page's syntax line never brackets the date argument either, and
* leaving it out entirely is error 1229 ("Too few arguments"), not today's quarter the way a
* lenient 0-argument default would give it.
LOCAL nGot
TRY
	nGot = QUARTER()
CATCH TO oErr
	? "QUARTER with no argument at all", oErr.ErrorNo
ENDTRY

* WEEK(): nFirstWeek changes which week a year's first few days of January land in when the
* year does not start on the first day of a week.
LOCAL ldJan1
ldJan1 = DATE(2022, 1, 1)
? "CDOW of that January 1st", CDOW(ldJan1)
? "WEEK rule 1 is the week holding January 1st", WEEK(ldJan1, 1)
? "WEEK rule 2 needs four days in the new year", WEEK(ldJan1, 2)

* COVERS: DATE, DATETIME, DTOC, QUARTER, WEEK
