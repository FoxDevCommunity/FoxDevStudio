* What happens when one name is defined twice.
*
* A form names a header file, and every method it holds is compiled as though that header's
* #DEFINEs came first - so a method that defines a name the header already defined is the second
* definition of it, and this says which of the two the code then sees. #UNDEF is the way out.
#DEFINE PICK "first"
#DEFINE PICK "second"
? PICK
#UNDEF PICK
#DEFINE PICK "third"
? PICK
* the header on the stage defines FDV_GREETING; a second definition here is the later one
#INCLUDE fdvgold.h
#DEFINE FDV_GREETING "from the program"
? FDV_GREETING
* a constant answers to any spelling of its name
#DEFINE MixedCase "either way"
? MIXEDCASE
? mixedcase

* COVERS: #DEFINE ... #UNDEF
