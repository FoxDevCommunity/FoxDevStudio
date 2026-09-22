* The date and time functions of the language reference, against a fixed date.
LOCAL d, t
d = {^2024-01-31}
t = {^2024-01-31 14:30:45}
? DTOC(d), DTOS(d)
? YEAR(d), MONTH(d), DAY(d)
? DOW(d), CDOW(d), CMONTH(d)
? WEEK(d)
? GOMONTH(d, 1), GOMONTH(d, -1)
? d + 1, d - 1, {^2024-03-01} - d
? HOUR(t), MINUTE(t), SEC(t)
? TTOC(t), TTOD(t)
? DTOT(d)
? CTOD("01/31/2024") = d
? CTOT("01/31/2024 02:30:45 PM") = t
SET CENTURY ON
? DTOC(d)
SET CENTURY OFF
SET DATE TO BRITISH
? DTOC(d)
SET DATE TO AMERICAN
? EMPTY({}), EMPTY(d)
? VARTYPE(d), VARTYPE(t)
* COVERS: CDOW, CMONTH, CTOD, CTOT, DAY, DOW, DTOC, DTOS, DTOT, EMPTY, GOMONTH, HOUR, LOCAL,
* COVERS: MINUTE, MONTH, SEC, SET CENTURY, SET DATE, TTOC, TTOD, VARTYPE, WEEK, YEAR
