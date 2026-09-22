* A Visual FoxPro library, loaded and called the way a program loads and calls one.
*
* vfpencryption71.fll is an ordinary .fll: a Win32 library that exports @DispatchAPI@4, takes
* the FoxPro API table the host hands it and answers with a table of the functions it adds to
* the language. A program says SET LIBRARY TO and then calls Hash() and Encrypt() as if they
* had always been there.
*
* Every number below was read out of the product with this library loaded - the SHA-1 of
* "hello world" is 2AAE6C35C94FCFB415DBE95F408B9CE91EE846ED, and so on for each of the eight
* algorithms the library offers. Nothing here is what the library's documentation says; it is
* what the library, in Visual FoxPro, on this machine, answered.
* private rather than local, because the procedure that counts them is called from here
PRIVATE lnPassed, lnFailed
lnPassed = 0
lnFailed = 0

SET LIBRARY TO vfpencryption71.fll ADDITIVE

* the eight digests, by their length and their bytes
DO Check WITH "hash 1 SHA-1", Hex(Hash("hello world", 1)), "2AAE6C35C94FCFB415DBE95F408B9CE91EE846ED"
DO Check WITH "hash 2 SHA-256", Hex(Hash("hello world", 2)), "B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9"
DO Check WITH "hash 3 SHA-384", Hex(Hash("hello world", 3)), "FDBD8E75A67F29F701A4E040385E2E23986303EA10239211AF907FCBB83578B3E417CB71CE646EFD0819DD8C088DE1BD"
DO Check WITH "hash 4 SHA-512", Hex(Hash("hello world", 4)), "309ECC489C12D6EB4CC40F50C902F2B4D0ED77EE511A7C7A9BCD3CA86D4CD86F989DD35BC5FF499670DA34255B45B0CFD830E81F605DCF7DC5542E93AE9CD76F"
DO Check WITH "hash 5 MD5", Hex(Hash("hello world", 5)), "5EB63BBBE01EEED093CB22BB8F5ACDC3"
DO Check WITH "hash 6", Hex(Hash("hello world", 6)), "C52AC4D06245286B33953957BE6C6F81"
DO Check WITH "hash 7", Hex(Hash("hello world", 7)), "98C615784CCB5FE5936FBC0CBE9DFDB408D92F0F"
DO Check WITH "hash 8", Hex(Hash("hello world", 8)), "309ECC489C12D6EB4CC40F50C902F2B4D0ED77EE511A7C7A9BCD3CA86D4CD86F989DD35BC5FF499670DA34255B45B0CFD830E81F605DCF7DC5542E93AE9CD76F"

* the ciphers: what one answers, and that what it answers can be read back
DO Check WITH "encrypt 1 length", TRANSFORM(LEN(Encrypt("hello world", "secretkey", 1))), "32"
DO Check WITH "encrypt 4 length", TRANSFORM(LEN(Encrypt("hello world", "secretkey", 4))), "16"
DO Check WITH "encrypt 4 bytes", Hex(Encrypt("hello world", "secretkey", 4)), "21CA401E56874A2715042EE5842AC7D9"
DO Check WITH "encrypt 8 bytes", Hex(Encrypt("hello world", "secretkey", 8)), "BBA477BE59CB6FDC32D723563A99AD9E"

* A block cipher pads what it is given to a whole block, and the padding is zero bytes rather
* than spaces - so what comes back is the text with CHR(0) after it, which the caller strips.
DO Check WITH "round trip 1", Unpad(Decrypt(Encrypt("hello world", "secretkey", 1), "secretkey", 1)), "hello world"
DO Check WITH "round trip 4", Unpad(Decrypt(Encrypt("hello world", "secretkey", 4), "secretkey", 4)), "hello world"
DO Check WITH "round trip 8", Unpad(Decrypt(Encrypt("hello world", "secretkey", 8), "secretkey", 8)), "hello world"

* a different key gives different bytes, which is the whole point of the key
DO Check WITH "key matters", IIF(Encrypt("hello world", "one", 4) == Encrypt("hello world", "two", 4), "same", "different"), "different"

? "PASSED " + TRANSFORM(lnPassed) + " FAILED " + TRANSFORM(lnFailed)

PROCEDURE Check
LPARAMETERS cWhat, cGot, cWant
IF cGot == cWant
    ? "[PASS] " + cWhat
    lnPassed = lnPassed + 1
ELSE
    ? "[FAIL] " + cWhat + ": " + cGot + " is not " + cWant
    lnFailed = lnFailed + 1
ENDIF
ENDPROC

FUNCTION Unpad
LPARAMETERS tc
RETURN STRTRAN(tc, CHR(0), "")
ENDFUNC

FUNCTION Hex
LPARAMETERS tc
LOCAL i, cOut
cOut = ""
FOR i = 1 TO LEN(tc)
    cOut = cOut + RIGHT("0" + TRANSFORM(ASC(SUBSTR(tc, i, 1)), "@0"), 2)
ENDFOR
RETURN cOut
ENDFUNC
