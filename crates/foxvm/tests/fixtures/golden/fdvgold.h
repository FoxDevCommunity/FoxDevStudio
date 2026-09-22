* A header file for the goldens, so #INCLUDE has something to read. The golden runner hands
* every .h in this directory to the compiler; scripts/vfp-expected.mjs copies them beside the
* program it gives Visual FoxPro, so both are asked the same question.
#DEFINE FDV_GREETING "hello from the header"
#DEFINE FDV_ANSWER 42
#DEFINE FDV_ON .T.
* a header may bring another one's constants with it
#INCLUDE fdvmore.h
