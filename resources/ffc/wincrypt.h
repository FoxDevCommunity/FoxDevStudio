#INCLUDE foxpro.h

*** Microsoft Windows Crypt API Constansts

***
*** Algorithm Classes
***
#DEFINE dnALG_CLASS_ANY          0
#DEFINE dnALG_CLASS_SIGNATURE    BITLSHIFT(1,13)
#DEFINE dnALG_CLASS_MSG_ENCRYPT  BITLSHIFT(2,13)
#DEFINE dnALG_CLASS_DATA_ENCRYPT BITLSHIFT(3,13)
#DEFINE dnALG_CLASS_HASH         BITLSHIFT(4,13)
#DEFINE dnALG_CLASS_KEY_EXCHANGE BITLSHIFT(5,13)

***
*** Algorithm Types
***
#DEFINE dnALG_TYPE_ANY 		 0
#DEFINE dnALG_TYPE_DSS           BITLSHIFT(1,9)
#DEFINE dnALG_TYPE_RSA           BITLSHIFT(2,9)
#DEFINE dnALG_TYPE_BLOCK         BITLSHIFT(3,9)
#DEFINE dnALG_TYPE_STREAM        BITLSHIFT(4,9)

***
*** Generic Sub-ID
***
#DEFINE dnALG_SID_ANY            0

***
*** Some RSA sub-ids
***
#DEFINE dnALG_SID_RSA_ANY        0
#DEFINE dnALG_SID_RSA_PKCS       1
#DEFINE dnALG_SID_RSA_MSATWORK   2
#DEFINE dnALG_SID_RSA_ENTRUST    3
#DEFINE dnALG_SID_RSA_PGP        4

***
*** Some DSS sub-ids
***
#DEFINE dnALG_SID_DSS_ANY        0
#DEFINE dnALG_SID_DSS_PKCS       1
#DEFINE dnALG_SID_DSS_DMS        2

***
*** Block cipher sub ids
*** DES sub_ids
***
#DEFINE dnALG_SID_DES           1
#DEFINE dnALG_SID_3DES		3
#DEFINE dnALG_SID_DESX		4
#DEFINE dnALG_SID_IDEA		5
#DEFINE dnALG_SID_CAST		6
#DEFINE dnALG_SID_SAFERSK64	7
#DEFINE dnALD_SID_SAFERSK128	8

***
*** KP_MODE
***
#DEFINE dnCRYPT_MODE_CBCI	6	&& ANSI CBC Interleaved
#DEFINE dnCRYPT_MODE_CFBP	7	&& ANSI CFB Pipelined
#DEFINE dnCRYPT_MODE_OFBP	8	&& ANSI OFB Pipelined
#DEFINE dnCRYPT_MODE_CBCOFM	9	&& ANSI CBC + OF Masking
#DEFINE dnCRYPT_MODE_CBCOFMI	10	&& ANSI CBC + OFM Interleaved

***
*** RC2 sub-ids
***
#DEFINE dnALG_SID_RC2           2

***
*** Stream cipher sub-ids
***
#DEFINE dnALG_SID_RC4           1
#DEFINE dnALG_SID_SEAL          2

***
*** Hash sub ids
***
#DEFINE dnALG_SID_MD2           1
#DEFINE dnALG_SID_MD4           2
#DEFINE dnALG_SID_MD5           3
#DEFINE dnALG_SID_SHA           4
#DEFINE dnALG_SID_MAC           5
#DEFINE dnALG_SID_RIPEMD	6
#DEFINE dnALG_SID_RIPEMD160	7
#DEFINE dnALG_SID_SSL3SHAMD5	8

***
*** algorithm identifier definitions
***
#DEFINE dnCALG_MD2        BITOR(BITOR(dnALG_CLASS_HASH,dnALG_TYPE_ANY),dnALG_SID_MD2)
#DEFINE dnCALG_MD4        BITOR(BITOR(dnALG_CLASS_HASH,dnALG_TYPE_ANY),dnALG_SID_MD4)
#DEFINE dnCALG_MD5        BITOR(BITOR(dnALG_CLASS_HASH,dnALG_TYPE_ANY),dnALG_SID_MD5)
#DEFINE dnCALG_SHA        BITOR(BITOR(dnALG_CLASS_HASH,dnALG_TYPE_ANY),dnALG_SID_SHA)
#DEFINE dnCALG_MAC        BITOR(BITOR(dnALG_CLASS_HASH,dnALG_TYPE_ANY),dnALG_SID_MAC)
#DEFINE dnCALG_RSA_SIGN   BITOR(BITOR(dnALG_CLASS_SIGNATURE,dnALG_TYPE_RSA),dnALG_SID_RSA_ANY)
#DEFINE dnCALG_DSS_SIGN   BITOR(BITOR(dnALG_CLASS_SIGNATURE,dnALG_TYPE_DSS),dnALG_SID_DSS_ANY)
#DEFINE dnCALG_RSA_KEYX   BITOR(BITOR(dnALG_CLASS_KEY_EXCHANGE,dnALG_TYPE_RSA),dnALG_SID_RSA_ANY)
#DEFINE dnCALG_DES        BITOR(BITOR(dnALG_CLASS_DATA_ENCRYPT,dnALG_TYPE_BLOCK),dnALG_SID_DES)
#DEFINE dnCALG_RC2 	  BITOR(BITOR(dnALG_CLASS_DATA_ENCRYPT,dnALG_TYPE_BLOCK),dnALG_SID_RC2)
#DEFINE dnCALG_RC4 	  BITOR(BITOR(dnALG_CLASS_DATA_ENCRYPT,dnALG_TYPE_STREAM),dnALG_SID_RC4)
#DEFINE dnCALG_SEAL       BITOR(BITOR(dnALG_CLASS_DATA_ENCRYPT,dnALG_TYPE_STREAM),dnALG_SID_SEAL)

***
*** dwFlags definitions for CryptAquireContext
***
#DEFINE dnCRYPT_VERIFYCONTEXT     0xF0000000
#DEFINE dnCRYPT_NEWKEYSET         0x8
#DEFINE dnCRYPT_DELETEKEYSET      0x10

***
*** dwFlag definitions for CryptGenKey
***
#DEFINE dnCRYPT_EXPORTABLE        0x00000001
#DEFINE dnCRYPT_USER_PROTECTED    0x00000002
#DEFINE dnCRYPT_CREATE_SALT       0x00000004
#DEFINE dnCRYPT_UPDATE_KEY        0x00000008

***
*** exported key blob definitions
***
#DEFINE dnSIMPLEBLOB              0x1
#DEFINE dnPUBLICKEYBLOB           0x6
#DEFINE dnPRIVATEKEYBLOB          0x7

#DEFINE dnAT_KEYEXCHANGE          1
#DEFINE dnAT_SIGNATURE            2

#DEFINE dnCRYPT_USERDATA          1

***
*** dwParam
***
#DEFINE dnKP_IV                   1       && Initialization vector
#DEFINE dnKP_SALT                 2       && Salt value
#DEFINE dnKP_PADDING              3       && Padding values
#DEFINE dnKP_MODE                 4       && Mode of the cipher
#DEFINE dnKP_MODE_BITS            5       && Number of bits to feedback
#DEFINE dnKP_PERMISSIONS          6       && Key permissions DWORD
#DEFINE dnKP_ALGID                7       && Key algorithm
#DEFINE dnKP_BLOCKLEN             8       && Block size of the cipher

***
*** KP_PADDING
***
#DEFINE dnPKCS5_PADDING           1       && PKCS 5 (sec 6.2) padding method

***
*** KP_MODE
***
#DEFINE dnCRYPT_MODE_CBC          1       && Cipher block chaining
#DEFINE dnCRYPT_MODE_ECB          2       && Electronic code book
#DEFINE dnCRYPT_MODE_OFB          3       && Output feedback mode
#DEFINE dnCRYPT_MODE_CFB          4       && Cipher feedback mode
#DEFINE dnCRYPT_MODE_CTS          5       && Ciphertext stealing mode

***
*** KP_PERMISSIONS
***
#DEFINE dnCRYPT_ENCRYPT           0x0001  && Allow encryption
#DEFINE dnCRYPT_DECRYPT           0x0002  && Allow decryption
#DEFINE dnCRYPT_EXPORT            0x0004  && Allow key to be exported
#DEFINE dnCRYPT_READ              0x0008  && Allow parameters to be read
#DEFINE dnCRYPT_WRITE             0x0010  && Allow parameters to be set
#DEFINE dnCRYPT_MAC               0x0020  && Allow MACs to be used with key

#DEFINE dnHP_ALGID                0x0001  && Hash algorithm
#DEFINE dnHP_HASHVAL              0x0002  && Hash value
#DEFINE dnHP_HASHSIZE             0x0004  && Hash value size

***
*** CryptGetProvParam
***
#DEFINE dnPP_ENUMALGS             1
#DEFINE dnPP_ENUMCONTAINERS       2
#DEFINE dnPP_IMPTYPE              3
#DEFINE dnPP_NAME                 4
#DEFINE dnPP_VERSION              5
#DEFINE dnPP_CONTAINER            6

#DEFINE dnCRYPT_FIRST             1
#DEFINE dnCRYPT_NEXT              2

#DEFINE dnCRYPT_IMPL_HARDWARE     1
#DEFINE dnCRYPT_IMPL_SOFTWARE     2
#DEFINE dnCRYPT_IMPL_MIXED        3
#DEFINE dnCRYPT_IMPL_UNKNOWN      4

***
*** CryptSetProvParam
***
#DEFINE dnPP_CLIENT_HWND          1

#DEFINE dnPROV_RSA_FULL           1
#DEFINE dnPROV_RSA_SIG            2
#DEFINE dnPROV_DSS                3
#DEFINE dnPROV_FORTEZZA           4
#DEFINE dnPROV_MS_EXCHANGE        5
#DEFINE dnPROV_SSL                6

***
***STT defined Providers
***
#DEFINE dnPROV_STT_MER            7
#DEFINE dnPROV_STT_ACQ            8
#DEFINE dnPROV_STT_BRND           9
#DEFINE dnPROV_STT_ROOT           10
#DEFINE dnPROV_STT_ISS            11


#DEFINE dsMS_DEF_PROV_A       "Microsoft Base Cryptographic Provider v1.0"

#DEFINE dnMAXUIDLEN               64

#DEFINE dnCUR_BLOB_VERSION        2

*** Format Message Constants (Part of Window.h)
#DEFINE dnFORMAT_MESSAGE_ALLOCATE_BUFFER 0x00000100
#DEFINE dnFORMAT_MESSAGE_IGNORE_INSERTS  0x00000200
#DEFINE dnFORMAT_MESSAGE_FROM_STRING     0x00000400
#DEFINE dnFORMAT_MESSAGE_FROM_HMODULE    0x00000800
#DEFINE dnFORMAT_MESSAGE_FROM_SYSTEM     0x00001000
#DEFINE dnFORMAT_MESSAGE_ARGUMENT_ARRAY  0x00002000
#DEFINE dnFORMAT_MESSAGE_MAX_WIDTH_MASK  0x000000FF

#DEFINE dcCREATECRYPTKEYS_ERR_LOC	'Cryptographic Service Provider could not be found or'+chr(13)+chr(10)+'Failed to Create Key  Container or'+chr(13)+chr(10)+'Failed to Create Keys in CSP Container'
#DEFINE dcB	'B'
#DEFINE dcBR 'BR'
#DEFINE dcCompleted	'Completed'
#DEFINE dcCRYPTINITIALIZE_ERR_LOC	'Crypt Service Provider Failed to Acquire'
#DEFINE dcCRYPTSIGGENKEY_ERR_LOC	'Crypt Service Provider Failed to Create an Signature Key Container'
#DEFINE dcCRYPTSIGGETKEY_ERR_LOC	'Crypt Service Provider Failed to Return an Signature Key Container'
#DEFINE dcCRYPTEXCGENKEY_ERR_LOC	'Crypt Service Provider Failed to Create an Exchange Key Container'
#DEFINE dcCRYPTEXCGETKEY_ERR_LOC	'Crypt Service Provider Failed to Return an Exchange Key Container'
#DEFINE	dcCRYPTPROVTYP_ERR_LOC	'Crypt Service Provider Not Installed or Not Found'
#DEFINE dcCRYPTNEWKEYCONTAINER_ERR_LOC 'New Crypt Key Container Could not be Created'
#DEFINE dcCRYPTDELETEKEYCONTAINER_ERR_LOC 'Crypt Key Container Could not be Deleted'
#DEFINE dcCRYPTUSEKEYCONTAINER_ERR_LOC 'Crypt Key Container Could not be Opened for Use'
#DEFINE dcDecryptSessionFileParameter_ERR_LOC	'Must Enter an Encrypted File Name and a Decrypted File Name and'+chr(13)+chr(10)+'a Password'+chr(13)+chr(10)+'in a parameter of this Function'
#DEFINE	dcDecryptSessionFileNoFile_ERR_LOC	'Encrypted File Could Not be Opened'+chr(13)+chr(10)+'Please Enter a File that Exists and is Not Already Open'
#DEFINE dcDecryptSessionFileExists_ERR_LOC	'Decrypted File Exists' + chr(13)+chr(10) + 'Do you wish to OverWrite it?'
#DEFINE dcDecryptSessionFileCreate_ERR_LOC	'Decrypted File Could not be Created/Overwritten'
#DEFINE	dcDecryptSessionFileWrite_ERR_LOC	'Error Occured During Write of Decryption File'
#DEFINE dcDecryptSessionFileComplete_LOC	'File Decryption Completed'
#DEFINE dcEmpty ''
#DEFINE dcEncryptSessionFileParameter_ERR_LOC	'Must Enter a Decrypted File Name and an Encrypted File Name and'+chr(13)+chr(10)+'a Password'+chr(13)+chr(10)+'in a parameter of this Function'
#DEFINE	dcEncryptSessionFileNoFile_ERR_LOC	'Decrypted File Could Not be Opened'+chr(13)+chr(10)+'Please Enter a File that Exists and is Not Already Open'
#DEFINE dcEncryptSessionFileExists_ERR_LOC	'Encrypted File Exists' + chr(13)+chr(10) + 'Do you wish to OverWrite it?'
#DEFINE dcEncryptSessionFileCreate_ERR_LOC	'Encrypted File Could not be Created/Overwritten'
#DEFINE	dcEncryptSessionFileWrite_ERR_LOC	'Error Occured During Write of Encryption File'
#DEFINE dcEncryptSessionFileComplete_LOC	'File Encryption Completed'
#DEFINE dcError	'ERROR'
#DEFINE dcErrorCode	'Error Facility - '
#DEFINE dcErrorFacility	'Error Facility - '
#DEFINE	dcErrorIn	'Error in '
#DEFINE dcErrorSeverity	'Error Severity - '
#DEFINE dcErrorSigningData 'Error Signing Data'
#DEFINE dcFileExists 'File Exists'
#DEFINE dcFOXPRO_APP 'FOXPRO_APP'
#DEFINE dcgetcryptsessionkeyhandleparam 'B,S,BR,SR'
#DEFINE dcgetcryptsessionkeyhandleERR_LOC	" must be called with parameter picType = 'B' or 'S' or 'BR' or 'SR'"
#DEFINE	dcSignFileParamter_ERR_LOC	'Must Enter a Signature Reference, a Decrypted File Name, and'+chr(13)+chr(10)+'a password'+chr(13)+chr(10)+'as a parameter to This Function'
#DEFINE dcSignFileDecryptedFile_ERR_LOC	'Decrypted File Could not be Opened'
#DEFINE	dcSignFileHashObject_ERR_LOC	'Error Creating Hash Object for Signature'
#DEFINE	dcSignFileHashing_ERR_LOC	'Error Hashing Data'
#DEFINE dcS	'S'
#DEFINE dcSR	'SR'

