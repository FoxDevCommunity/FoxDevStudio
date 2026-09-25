/*
 * A library that writes to variables passed by reference, which neither of Microsoft's API
 * samples does. Built by scripts/build-fllhost.mjs beside them, with the same toolchain, and
 * measured against Visual FoxPro 9 in tests/fll/samples.test.ts.
 *
 *   SWAPREF(cNew, @cVar)  stores cNew in cVar and answers with what cVar held before
 *   BUMPREF(@nVar)        adds one to nVar and answers .T.
 *   SETREF(@uVar)         stores the integer 42 in uVar, whatever it held
 *   INTOF(n)              answers the integer the library was handed for n, declared "I"
 */

#include <pro_ext.h>

void FAR SwapRef(ParamBlk FAR *parm) {
  Value old;
  _Load(&parm->p[1].loc, &old);
  _Store(&parm->p[1].loc, &parm->p[0].val);
  if (old.ev_type == 'C') {
    _RetChar("");
    if (_SetHandSize(old.ev_handle, old.ev_length + 1)) {
      ((char FAR *)_HandToPtr(old.ev_handle))[old.ev_length] = '\0';
      _RetChar((char FAR *)_HandToPtr(old.ev_handle));
    }
    _FreeHand(old.ev_handle);
  } else {
    _RetLogical(0);
  }
}

void FAR BumpRef(ParamBlk FAR *parm) {
  Value v;
  _Load(&parm->p[0].loc, &v);
  if (v.ev_type == 'N') v.ev_real += 1;
  else if (v.ev_type == 'I') v.ev_long += 1;
  _Store(&parm->p[0].loc, &v);
  _RetLogical(1);
}

void FAR SetRef(ParamBlk FAR *parm) {
  Value v;
  v.ev_type = 'I';
  v.ev_width = 10;
  v.ev_long = 42;
  _Store(&parm->p[0].loc, &v);
  _RetLogical(1);
}

void FAR IntOf(ParamBlk FAR *parm) {
  _RetInt(parm->p[0].val.ev_long, 11);
}

FoxInfo myFoxInfo[] = {
    {"SWAPREF", (FPFI)SwapRef, 2, "C,R"},
    {"BUMPREF", (FPFI)BumpRef, 1, "R"},
    {"SETREF", (FPFI)SetRef, 1, "R"},
    {"INTOF", (FPFI)IntOf, 1, "I"},
};

FoxTable _FoxTable = {(FoxTable FAR *)0, sizeof(myFoxInfo) / sizeof(FoxInfo), myFoxInfo};
