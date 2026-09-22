* Reaching a data source that is not there: what every pass-through call answers.
? SQLCONNECT("nosuchsource")
? SQLSTRINGCONNECT("DSN=nosuchsource")
? SQLDISCONNECT(1)
? SQLEXEC(1, "SELECT 1")
? SQLTABLES(1)
? SQLCOLUMNS(1, "titles")
? SQLCOMMIT(1), SQLROLLBACK(1), SQLCANCEL(1)
? SQLMORERESULTS(1)

* what a connection is set to, which is kept whether or not one is open
? SQLGETPROP(1, "Transactions")
? SQLSETPROP(1, "Transactions", 2)
? SQLGETPROP(1, "Transactions")
? SQLSETPROP(1, "ConnectString", "DSN=books")
? SQLGETPROP(1, "ConnectString")
? SQLPREPARE(1, "SELECT * FROM titles")
? SQLGETPROP(1, "Prepared")
? SQLIDLEDISCONNECT(1), SQLIDLEDISCONNECT(0)

* COVERS: SQLCANCEL, SQLCOLUMNS, SQLCOMMIT, SQLCONNECT, SQLDISCONNECT, SQLEXEC, SQLGETPROP,
* COVERS: SQLIDLEDISCONNECT, SQLMORERESULTS, SQLPREPARE, SQLROLLBACK, SQLSETPROP,
* COVERS: SQLSTRINGCONNECT, SQLTABLES
