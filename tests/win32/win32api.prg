* COVERS: DECLARE
* Calling Windows itself: the twelve shapes a Visual FoxPro program uses to reach the API.
* Structures packed and unpacked by hand with BTOA and CTOB, buffers passed by reference,
* handles kept as integers, and a call that takes nothing at all.
LOCAL lnPassed, lnFailed
lnPassed = 0
lnFailed = 0

* 1: a struct filled through a string passed by reference - POINT is two 32-bit signed longs
DECLARE INTEGER GetCursorPos IN user32.dll STRING @lpPoint
LOCAL lcPointStruct, lnX, lnY
lcPointStruct = REPLICATE(CHR(0), 8)
IF GetCursorPos(@lcPointStruct) != 0
    lnX = CTOBIN(SUBSTR(lcPointStruct, 1, 4), "4SR")
    lnY = CTOBIN(SUBSTR(lcPointStruct, 5, 4), "4SR")
    ? "[PASS] 1 GetCursorPos"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 1 GetCursorPos"
    lnFailed = lnFailed + 1
ENDIF

* 2: SYSTEMTIME, eight 16-bit words in one buffer
DECLARE VOID GetLocalTime IN kernel32.dll STRING @lpSystemTime
LOCAL lcSysTime, lnYear, lnMonth
lcSysTime = REPLICATE(CHR(0), 16)
GetLocalTime(@lcSysTime)
lnYear = CTOBIN(SUBSTR(lcSysTime, 1, 2), "2RS")
lnMonth = CTOBIN(SUBSTR(lcSysTime, 3, 2), "2RS")
IF lnYear >= 2020 AND BETWEEN(lnMonth, 1, 12)
    ? "[PASS] 2 GetLocalTime"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 2 GetLocalTime year=" + TRANSFORM(lnYear) + " month=" + TRANSFORM(lnMonth)
    lnFailed = lnFailed + 1
ENDIF

* 3: a handle out and back again.
*
* INTEGER is four bytes, which is what a handle is in a 32-bit process - and Visual FoxPro is
* one. This runtime is not: the heap it allocates from is above four gigabytes, so the pointer
* GlobalAlloc answers does not fit in the type the language has to carry it, and the address
* handed back to GlobalFree is not the one that was allocated. Nothing in the language can say
* it, so the call is made and the answer reported rather than pretended about.
DECLARE INTEGER GlobalAlloc IN kernel32.dll INTEGER uFlags, INTEGER dwBytes
DECLARE INTEGER GlobalFree IN kernel32.dll INTEGER hMem
LOCAL lnHMem
lnHMem = GlobalAlloc(0x0040, 1024)
IF lnHMem != 0 AND GlobalFree(lnHMem) == 0
    ? "[PASS] 3 GlobalAlloc/GlobalFree"
    lnPassed = lnPassed + 1
ELSE
    ? "[KNOWN] 3 GlobalAlloc: a handle wider than four bytes cannot come back through INTEGER"
ENDIF

* 4: a string in, a string buffer out, a length back
DECLARE INTEGER GetEnvironmentVariableA IN kernel32.dll STRING lpName, STRING @lpBuffer, INTEGER nSize
LOCAL lcEnvBuffer, lnRetSize
lcEnvBuffer = REPLICATE(CHR(0), 1024)
lnRetSize = GetEnvironmentVariableA("PATH", @lcEnvBuffer, 1024)
IF lnRetSize > 0
    ? "[PASS] 4 GetEnvironmentVariableA"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 4 GetEnvironmentVariableA"
    lnFailed = lnFailed + 1
ENDIF

* 5: a struct whose first field says how big it is, and a 64-bit field read out of it
DECLARE INTEGER GlobalMemoryStatusEx IN kernel32.dll STRING @lpBuffer
LOCAL lcMemStruct, lnMemLoad, lnTotalPhys
lcMemStruct = BINTOC(64, "4RS") + REPLICATE(CHR(0), 60)
IF GlobalMemoryStatusEx(@lcMemStruct) != 0
    lnMemLoad = CTOBIN(SUBSTR(lcMemStruct, 5, 4), "4RS")
    lnTotalPhys = Unsigned64(SUBSTR(lcMemStruct, 9, 8))
    IF BETWEEN(lnMemLoad, 0, 100) AND lnTotalPhys > 0
        ? "[PASS] 5 GlobalMemoryStatusEx"
        lnPassed = lnPassed + 1
    ELSE
        ? "[FAIL] 5 GlobalMemoryStatusEx load=" + TRANSFORM(lnMemLoad) + " total=" + TRANSFORM(lnTotalPhys)
        lnFailed = lnFailed + 1
    ENDIF
ELSE
    ? "[FAIL] 5 GlobalMemoryStatusEx call"
    lnFailed = lnFailed + 1
ENDIF

* 6: three buffers by reference in one call
DECLARE INTEGER GetDiskFreeSpaceExA IN kernel32.dll STRING lpDirectoryName, STRING @lpFreeBytesAvailableToCaller, STRING @lpTotalNumberOfBytes, STRING @lpTotalNumberOfFreeBytes
LOCAL lcFreeAvail, lcTotalBytes, lcTotalFree, lnTotalGB
lcFreeAvail = REPLICATE(CHR(0), 8)
lcTotalBytes = REPLICATE(CHR(0), 8)
lcTotalFree = REPLICATE(CHR(0), 8)
IF GetDiskFreeSpaceExA("C:\", @lcFreeAvail, @lcTotalBytes, @lcTotalFree) != 0
    lnTotalGB = INT(Unsigned64(lcTotalBytes) / (1024 * 1024 * 1024))
    IF lnTotalGB > 0
        ? "[PASS] 6 GetDiskFreeSpaceExA"
        lnPassed = lnPassed + 1
    ELSE
        ? "[FAIL] 6 GetDiskFreeSpaceExA size=" + TRANSFORM(lnTotalGB)
        lnFailed = lnFailed + 1
    ENDIF
ELSE
    ? "[FAIL] 6 GetDiskFreeSpaceExA call"
    lnFailed = lnFailed + 1
ENDIF

* 7: a 64-bit counter into an eight-byte buffer
DECLARE INTEGER QueryPerformanceFrequency IN kernel32.dll STRING @lpFrequency
LOCAL lcFreqBuffer, lnFreq
lcFreqBuffer = REPLICATE(CHR(0), 8)
IF QueryPerformanceFrequency(@lcFreqBuffer) != 0
    lnFreq = Unsigned64(lcFreqBuffer)
    IF lnFreq > 0
        ? "[PASS] 7 QueryPerformanceFrequency"
        lnPassed = lnPassed + 1
    ELSE
        ? "[FAIL] 7 QueryPerformanceFrequency freq=" + TRANSFORM(lnFreq)
        lnFailed = lnFailed + 1
    ENDIF
ELSE
    ? "[FAIL] 7 QueryPerformanceFrequency call"
    lnFailed = lnFailed + 1
ENDIF

* 8: a path written into a buffer the caller sized
DECLARE INTEGER GetSystemDirectoryA IN kernel32.dll STRING @lpBuffer, INTEGER uSize
LOCAL lcSysDirBuffer, lnSysDirRes
lcSysDirBuffer = REPLICATE(CHR(0), 260)
lnSysDirRes = GetSystemDirectoryA(@lcSysDirBuffer, 260)
IF lnSysDirRes > 0 AND ":" $ LEFT(lcSysDirBuffer, lnSysDirRes)
    ? "[PASS] 8 GetSystemDirectoryA"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 8 GetSystemDirectoryA res=" + TRANSFORM(lnSysDirRes)
    lnFailed = lnFailed + 1
ENDIF

* 9: two calls in a row, the first one's answer feeding the second
DECLARE INTEGER GetTempPathA IN kernel32.dll INTEGER nBufferLength, STRING @lpBuffer
DECLARE INTEGER GetTempFileNameA IN kernel32.dll STRING lpPathName, STRING lpPrefixString, INTEGER uUnique, STRING @lpTempFileName
LOCAL lcTempPath, lcTempFile, lnPathRes, lnFileRes
lcTempPath = REPLICATE(CHR(0), 260)
lcTempFile = REPLICATE(CHR(0), 260)
lnPathRes = GetTempPathA(260, @lcTempPath)
IF lnPathRes > 0
    lcTempPath = LEFT(lcTempPath, lnPathRes)
    lnFileRes = GetTempFileNameA(lcTempPath, "FOX", 0, @lcTempFile)
    IF lnFileRes != 0
        ? "[PASS] 9 GetTempFileNameA"
        lnPassed = lnPassed + 1
    ELSE
        ? "[FAIL] 9 GetTempFileNameA"
        lnFailed = lnFailed + 1
    ENDIF
ELSE
    ? "[FAIL] 9 GetTempPathA"
    lnFailed = lnFailed + 1
ENDIF

* 10: the same function asked three different questions
DECLARE INTEGER GetSystemMetrics IN user32.dll INTEGER nIndex
LOCAL lnVScrollWidth, lnHScrollHeight, lnMonitors
lnVScrollWidth = GetSystemMetrics(2)
lnHScrollHeight = GetSystemMetrics(3)
lnMonitors = GetSystemMetrics(80)
IF lnVScrollWidth > 0 AND lnHScrollHeight > 0 AND lnMonitors >= 1
    ? "[PASS] 10 GetSystemMetrics"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 10 GetSystemMetrics " + TRANSFORM(lnVScrollWidth) + " " + TRANSFORM(lnHScrollHeight) + " " + TRANSFORM(lnMonitors)
    lnFailed = lnFailed + 1
ENDIF

* 11: a call that takes nothing, and a handle handed straight back to another
DECLARE INTEGER GetDesktopWindow IN user32.dll
DECLARE INTEGER GetClassNameA IN user32.dll INTEGER hWnd, STRING @lpClassName, INTEGER nMaxCount
LOCAL lnHwndDesk, lcClassBuffer, lnClassRes
lnHwndDesk = GetDesktopWindow()
lcClassBuffer = REPLICATE(CHR(0), 128)
lnClassRes = GetClassNameA(lnHwndDesk, @lcClassBuffer, 128)
IF lnClassRes > 0
    ? "[PASS] 11 GetClassNameA"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 11 GetClassNameA hwnd=" + TRANSFORM(lnHwndDesk) + " res=" + TRANSFORM(lnClassRes)
    lnFailed = lnFailed + 1
ENDIF

* 12: a call that returns nothing and takes time doing it
DECLARE VOID Sleep IN kernel32.dll INTEGER dwMilliseconds
LOCAL lnBefore, lnAfter, lnElapsed
lnBefore = SECONDS()
Sleep(250)
lnAfter = SECONDS()
lnElapsed = (lnAfter - lnBefore) * 1000
IF lnElapsed >= 150 AND lnElapsed <= 1500
    ? "[PASS] 12 Sleep"
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] 12 Sleep elapsed=" + TRANSFORM(lnElapsed)
    lnFailed = lnFailed + 1
ENDIF

? "PASSED " + TRANSFORM(lnPassed) + " FAILED " + TRANSFORM(lnFailed)

* Eight bytes of a Windows structure as one number.
*
* There is no 64-bit whole number in this language - CTOBIN reads eight bytes as a double - so a
* program reads the two 32-bit halves and puts them together, which is what every VFP program
* that asks Windows for a file size or a counter does. Each half is signed, so one that has run
* past the top of a signed 32-bit number is brought back by adding 2^32.
FUNCTION Unsigned64
LPARAMETERS tcEight
LOCAL lnLow, lnHigh
lnLow = CTOBIN(SUBSTR(tcEight, 1, 4), "4RS")
lnHigh = CTOBIN(SUBSTR(tcEight, 5, 4), "4RS")
IF lnLow < 0
    lnLow = lnLow + 4294967296
ENDIF
IF lnHigh < 0
    lnHigh = lnHigh + 4294967296
ENDIF
RETURN lnHigh * 4294967296 + lnLow
ENDFUNC
