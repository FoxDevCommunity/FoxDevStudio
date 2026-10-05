/*
 * fllhost - the process that hosts Visual FoxPro libraries.
 *
 * A .fll is not a .dll a program declares functions out of. It is a library written against
 * FoxPro's own C API: the host hands it a table of API services, and it answers with a table of
 * the functions it adds to the language. `SET LIBRARY TO` loads it and its functions are then
 * called as though they had always been there.
 *
 * Every .fll ever shipped is a 32-bit Win32 DLL, and Node is 64-bit, so nothing in the
 * application's own processes can load one: a 64-bit process cannot map a 32-bit image, and no
 * amount of FFI changes that. This program is therefore built for x86 and spoken to over its
 * standard input and output by the main process. It is the only place in the product where a
 * foreign pointer is dereferenced, and it is a separate process precisely so that a library
 * that walks off the end of something takes this down and not the IDE.
 *
 * The protocol between a library and its host was read out of winapims.lib, the static library
 * every .fll is linked against (`lib /extract:d:\vfp90\obj\apiobj\apimain.obj`, then
 * `dumpbin -disasm`). Nothing here is guessed from documentation. What that disassembly says:
 *
 *   The library exports one symbol, `@DispatchAPI@4`, __fastcall, one pointer in ECX. That
 *   pointer is a request block. Byte 0x14 of it is a command:
 *
 *     0   initialise. The host puts its API callback at +0x18, its handle table at +0x00 and
 *         its object table at +0x04; the library remembers all three in thread-local storage
 *         and answers with the address of its FoxTable at +0x00 and 5000 - its API version -
 *         at +0x04.
 *     1   call one of the library's functions. The host puts the function's address at +0x18
 *         and builds a ParamBlk at +0x24; the library calls it with ECX pointing at that block.
 *     2   allocate a memory handle. Size at +0x00, attribute in the word at +0x04, handle back
 *         at +0x00. The library's own allocator owns handles, which is why the host has to ask.
 *     4   free the handle at +0x00.
 *     6   turn the handle at +0x00 into a pointer, back at +0x00.
 *     10  call the function at +0x18 with the two words at +0x00 and +0x04.
 *     11  call the function at +0x18 with +0x00 and the address of +0x24.
 *     14  answer with the FoxTable and version again, without re-initialising.
 *     16, 17, 18  allocate, free and dereference an object slot.
 *
 *   Everything the library asks of the host goes through the single callback it was given at
 *   command 0. It is called __fastcall with ECX pointing at a small block: the first four
 *   arguments at +0x00, +0x04, +0x08 and +0x0C, a fifth at +0x18, which API function is wanted
 *   at +0x10, and the answer written back over +0x00. The numbers at +0x10 are the ones the
 *   thunks in winapims.lib push - _PutStr is 2, _RetVal is 11, _Error is 83 - and this is why a
 *   library imports nothing from vfp9.exe: there is one dispatcher, not a table of a hundred
 *   entries with offsets to get wrong.
 *
 *   The two numbers that matter beyond that: after the host's callback returns from _Error (83)
 *   or _UserError (84) the library's own stub longjmps out of the function that called it, so
 *   those two never return to the library and the host does not have to arrange it.
 */

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <fcntl.h>
#include <io.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---------------------------------------------------------------- the blocks -------- */

#pragma pack(push, 1)

/*
 * The library's own view of these, copied from pro_ext.h - the header Microsoft ships with
 * Visual FoxPro, in Samples\API - so that this program builds with nothing but a compiler.
 * pro_ext.h packs its structures to a byte boundary and this must agree with it exactly: a
 * Value is 36 bytes, a FoxInfo 14, a FoxTable 10.
 */
#define FASTCALL __fastcall
typedef long(FAR *FPFI)();

typedef struct {
  char ev_type;
  char ev_padding;
  short ev_width;
  unsigned ev_length;
  long ev_long;
  double ev_real;
  LARGE_INTEGER ev_currency;
  unsigned ev_handle;
  unsigned long ev_object;
} Value;

typedef struct {
  char l_type;
  SHORT l_where;
  USHORT l_NTI, l_offset, l_subs, l_sub1, l_sub2;
} Locator;

typedef union {
  Value val;
  Locator loc;
} FoxParameter;

typedef struct {
  char FAR *funcName;
  FPFI function;
  short parmCount; /* or one of the three flags below */
  char FAR *parmTypes;
} FoxInfo;

typedef struct _FoxTable {
  struct _FoxTable FAR *nextLibrary;
  short infoCount;
  FoxInfo FAR *infoPtr;
} FoxTable;

#define INTERNAL (-1)     /* not callable from FoxPro */
#define CALLONLOAD (-2)   /* run when the library is loaded */
#define CALLONUNLOAD (-3) /* run when it is unloaded */

#define FLL_PATH 261 /* pro_ext.h's MAX_PATH */

/*
 * The request block handed to @DispatchAPI@4. The fields named here are the ones the
 * disassembly reads or writes; the gaps between them it never touches, so they are left alone
 * rather than guessed at.
 */
#define MAX_PARMS 32
typedef struct {
  void *slot0;    /* +0x00 first argument, and where an answer comes back */
  unsigned slot1; /* +0x04 second argument, and the version at command 0 */
  unsigned char gap1[0x0c];
  unsigned cmd; /* +0x14 */
  void *fn;     /* +0x18 the function to call, or the host callback at command 0 */
  unsigned char gap2[8];
  short pCount;              /* +0x24 ParamBlk.pCount */
  FoxParameter p[MAX_PARMS]; /* +0x26 ParamBlk.p */
} Request;

/*
 * What the library's stub passes to the host callback. Read off @xWinCall@8 and its wider
 * cousins, which build it on their own stack: the arguments land at 0, 4, 8, 0x0c and 0x18,
 * the API function number at 0x10, and the answer is read back out of offset 0.
 */
typedef struct {
  unsigned a; /* +0x00 first argument; the return value goes back here */
  unsigned b; /* +0x04 */
  unsigned c; /* +0x08 */
  unsigned d; /* +0x0c */
  unsigned code;
  unsigned flag; /* +0x14 the stub clears this before it calls */
  unsigned e;    /* +0x18 fifth argument */
} ApiCall;

/*
 * The handle table the host owns and the library's allocator works out of: a cursor, a
 * capacity, and one HGLOBAL per slot. A handle is the slot's index, so index 0 is never handed
 * out - _FreeHand and _HandToPtr both treat 0 as nothing.
 */
#define HANDLE_SLOTS 8192
typedef struct {
  unsigned short cursor;
  unsigned short capacity;
  HGLOBAL slot[HANDLE_SLOTS];
} HandleTable;

/* The same shape for objects, with the count in use kept in the second word. */
#define OBJECT_SLOTS 1024
typedef struct {
  unsigned short cursor;
  unsigned short inUse;
  unsigned short capacity;
  void *slot[OBJECT_SLOTS];
} ObjectTable;

#pragma pack(pop)

/* Commands understood by @DispatchAPI@4. */
#define CMD_INIT 0
#define CMD_CALL 1
#define CMD_ALLOC 2
#define CMD_FREE 4
#define CMD_DEREF 6

/* Which API function the library is asking for; the numbers the winapims.lib thunks push. */
#define API_PUTCHR 0
#define API_PUTSTR 2
#define API_RETDATESTR 10
#define API_RETVAL 11
#define API_STORE 29 /* _Store(Locator *, Value *): write a variable passed by reference */
#define API_LOAD 30  /* _Load(Locator *, Value *): read one */
#define API_PUTVALUE 64
#define API_ERROR 83
#define API_USERERROR 84
#define API_RETDATETIMESTR 120

/* The attribute _AllocHand writes into the two bytes in front of every block it allocates. */
#define HANDLE_ATTR 0x10

/* ---------------------------------------------------------------- libraries --------- */

typedef void(FASTCALL *DispatchFn)(void *block);

typedef struct {
  HMODULE module;
  DispatchFn dispatch;
  Request *req; /* initialising, and calling a function */
  Request *mem; /* asking for memory, which happens while a call is in flight */
  HandleTable *handles;
  ObjectTable *objects;
  FoxInfo **funcs;
  int count;
  char path[FLL_PATH];
} Library;

#define MAX_LIBRARIES 64
static Library g_lib[MAX_LIBRARIES];

/* The library a call is running in: the handles in a returned Value belong to its table. */
static Library *g_current;

static void *alloc_handle(Library *lib, unsigned size, unsigned *handle);
static void free_handle(Library *lib, unsigned handle);
static void *deref_handle(Library *lib, unsigned handle);

/* ---------------------------------------------------------------- what a call said --- */

static char *g_out;
static size_t g_outLen, g_outCap;
static Value g_ret;
static int g_hasRet;
/* the bytes of a character answer, taken while the library was still holding them */
static char *g_retText;
static size_t g_retTextLen, g_retTextCap;
static int g_errNo;
static char g_errText[512];
static char g_missing[256];

/*
 * The variables a call was handed by reference, by parameter position. A library is given a
 * Locator for each - `cmRegGetValue(nRoot, cKey, @cValue)` is declared "I,C,R" - and reads and
 * writes the variable through _Load and _Store. What it stored goes back with the answer, and
 * the runtime writes it into the variable. A character value is kept as its bytes, because the
 * handle a library stores from is its own to free the moment _Store returns.
 */
static Value g_refs[MAX_PARMS];
static char *g_refText[MAX_PARMS];
static unsigned g_refTextLen[MAX_PARMS];
static int g_refIs[MAX_PARMS], g_refStored[MAX_PARMS];

static void ref_keep_text(unsigned i, const char *p, unsigned len) {
  char *grown = (char *)realloc(g_refText[i], len + 1);
  if (!grown) return;
  g_refText[i] = grown;
  if (p && len) memcpy(grown, p, len);
  grown[len] = '\0';
  g_refTextLen[i] = len;
}

static void say(const char *bytes, size_t len) {
  if (g_outLen + len + 1 > g_outCap) {
    size_t want = (g_outLen + len + 1) * 2;
    char *grown = (char *)realloc(g_out, want);
    if (!grown) return;
    g_out = grown;
    g_outCap = want;
  }
  memcpy(g_out + g_outLen, bytes, len);
  g_outLen += len;
}

/* Records an API function we were asked for and do not have, so the caller is told rather than
   left with a wrong answer. */
static void missing(unsigned code) {
  char one[16];
  sprintf(one, "%s%u", g_missing[0] ? "," : "", code);
  if (strlen(g_missing) + strlen(one) + 1 < sizeof(g_missing)) strcat(g_missing, one);
}

/* ---------------------------------------------------------------- the API ----------- */

/*
 * Everything a library asks of its host arrives here. The library is holding our stack while
 * this runs, so it does nothing that can fail: it copies, records, and answers.
 */
static void FASTCALL host_api(ApiCall *c) {
  switch (c->code) {
    case API_PUTSTR: {
      const char *s = (const char *)c->a;
      if (s) say(s, strlen(s));
      c->a = 0;
      break;
    }
    case API_PUTCHR: {
      char ch = (char)c->a;
      say(&ch, 1);
      c->a = 0;
      break;
    }
    case API_PUTVALUE: {
      /* _PutValue prints a value the way ? does. Only a string can be printed without the
         runtime's own formatting rules, so that is all this does; anything else is recorded. */
      Value *v = (Value *)c->a;
      if (v && (v->ev_type == 'C' || v->ev_type == 'H') && g_current) {
        Request *r = g_current->req;
        r->cmd = CMD_DEREF;
        r->slot0 = (void *)v->ev_handle;
        g_current->dispatch(r);
        if (r->slot0) say((const char *)r->slot0, v->ev_length);
      } else {
        missing(c->code);
      }
      c->a = 0;
      break;
    }
    case API_RETVAL:
    case API_RETDATESTR:
    case API_RETDATETIMESTR: {
      /* _RetChar, _RetInt, _RetFloat and the rest all build a Value on their own stack and
         hand it over as _RetVal; the two date calls hand over a string instead. */
      if (c->code == API_RETVAL) {
        Value *v = (Value *)c->a;
        if (v) {
          g_ret = *v;
          g_hasRet = 1;
          /*
           * A character answer has to be taken now, not after the function returns. Measured
           * on vfpencryption71.fll: HASH allocates a handle, fills it with the digest, hands
           * the Value to _RetVal and then frees the handle on the very next instruction. The
           * host that reads it afterwards reads freed memory - which is why this copies the
           * bytes here and lets the handle go itself. The library's own _FreeHand afterwards
           * finds the slot already empty and does nothing, so both orders are safe.
           */
          if ((v->ev_type == 'C' || v->ev_type == 'H') && g_current) {
            const char *p = (const char *)deref_handle(g_current, v->ev_handle);
            if (v->ev_length + 1 > g_retTextCap) {
              char *grown = (char *)realloc(g_retText, v->ev_length + 1);
              if (grown) {
                g_retText = grown;
                g_retTextCap = v->ev_length + 1;
              }
            }
            g_retTextLen = 0;
            if (p && g_retTextCap > v->ev_length) {
              memcpy(g_retText, p, v->ev_length);
              g_retTextLen = v->ev_length;
            }
            free_handle(g_current, v->ev_handle);
          }
        }
      } else {
        const char *s = (const char *)c->a;
        memset(&g_ret, 0, sizeof(g_ret));
        g_ret.ev_type = c->code == API_RETDATESTR ? 'd' : 't';
        g_ret.ev_length = s ? (unsigned)strlen(s) : 0;
        g_ret.ev_long = (long)c->a;
        g_hasRet = 1;
      }
      c->a = 0;
      break;
    }
    case API_STORE:
    case API_LOAD: {
      Locator *loc = (Locator *)c->a;
      Value *v = (Value *)c->b;
      unsigned i = loc ? (unsigned)loc->l_NTI - 1 : MAX_PARMS;
      if (!v || i >= MAX_PARMS || !g_refIs[i] || !g_current) {
        c->a = (unsigned)-1;
        break;
      }
      if (c->code == API_STORE) {
        g_refs[i] = *v;
        if (v->ev_type == 'C' || v->ev_type == 'H') {
          g_refs[i].ev_type = 'C';
          ref_keep_text(i, (const char *)deref_handle(g_current, v->ev_handle), v->ev_length);
        }
        g_refStored[i] = 1;
      } else {
        *v = g_refs[i];
        /* a character value comes with a handle of its own, which the library frees */
        if (v->ev_type == 'C') {
          unsigned handle = 0;
          void *p = alloc_handle(g_current, g_refTextLen[i] + 1, &handle);
          if (p) memcpy(p, g_refText[i] ? g_refText[i] : "", g_refTextLen[i] + 1);
          v->ev_handle = handle;
          v->ev_length = g_refTextLen[i];
        }
      }
      c->a = 0;
      break;
    }
    case API_ERROR: {
      /* The library's own stub longjmps out of the function as soon as this returns, so the
         number is all there is to keep. */
      g_errNo = (int)c->a;
      c->a = 0;
      break;
    }
    case API_USERERROR: {
      const char *s = (const char *)c->a;
      g_errNo = -1;
      if (s) {
        size_t n = strlen(s);
        if (n >= sizeof(g_errText)) n = sizeof(g_errText) - 1;
        memcpy(g_errText, s, n);
        g_errText[n] = '\0';
      }
      c->a = 0;
      break;
    }
    default:
      missing(c->code);
      c->a = 0;
      break;
  }
}

/* ---------------------------------------------------------------- framing ----------- */

static unsigned char *g_in;
static size_t g_inLen, g_inPos;
static unsigned char *g_reply;
static size_t g_replyLen, g_replyCap;

/*
 * The channel. A named pipe when one was named on the command line, standard input and output
 * otherwise. The pipe exists because the runtime has to be able to call a library function in
 * the middle of evaluating an expression - TYPE([Hash("a", 5)]) is the smallest case - and Node
 * has no way to wait for a child process's pipe without giving up the stack. A named pipe it
 * opened as a file it can read and write with fs.readSync and fs.writeSync, which block.
 */
static HANDLE g_pipe = INVALID_HANDLE_VALUE;

static int read_exact(void *into, size_t n) {
  unsigned char *p = (unsigned char *)into;
  while (n) {
    if (g_pipe != INVALID_HANDLE_VALUE) {
      DWORD got = 0;
      if (!ReadFile(g_pipe, p, (DWORD)n, &got, NULL) || got == 0) return 0;
      p += got;
      n -= got;
    } else {
      size_t got = fread(p, 1, n, stdin);
      if (got == 0) return 0;
      p += got;
      n -= got;
    }
  }
  return 1;
}

static void write_all(const void *from, size_t n) {
  const unsigned char *p = (const unsigned char *)from;
  if (g_pipe != INVALID_HANDLE_VALUE) {
    while (n) {
      DWORD sent = 0;
      if (!WriteFile(g_pipe, p, (DWORD)n, &sent, NULL) || sent == 0) return;
      p += sent;
      n -= sent;
    }
    return;
  }
  fwrite(p, 1, n, stdout);
  fflush(stdout);
}

static void put(const void *bytes, size_t n) {
  if (g_replyLen + n > g_replyCap) {
    size_t want = (g_replyLen + n) * 2 + 64;
    unsigned char *grown = (unsigned char *)realloc(g_reply, want);
    if (!grown) exit(3);
    g_reply = grown;
    g_replyCap = want;
  }
  memcpy(g_reply + g_replyLen, bytes, n);
  g_replyLen += n;
}
static void put_u8(unsigned v) {
  unsigned char b = (unsigned char)v;
  put(&b, 1);
}
static void put_u16(unsigned v) {
  unsigned short b = (unsigned short)v;
  put(&b, 2);
}
static void put_i32(int v) { put(&v, 4); }
static void put_u32(unsigned v) { put(&v, 4); }
static void put_f64(double v) { put(&v, 8); }
static void put_bytes32(const void *b, size_t n) {
  put_u32((unsigned)n);
  put(b, n);
}
static void put_text(const char *s) {
  size_t n = s ? strlen(s) : 0;
  put_u16((unsigned)n);
  put(s, n);
}

static int take(void *into, size_t n) {
  if (g_inPos + n > g_inLen) return 0;
  memcpy(into, g_in + g_inPos, n);
  g_inPos += n;
  return 1;
}
static unsigned take_u16(void) {
  unsigned short v = 0;
  take(&v, 2);
  return v;
}
static unsigned take_u32(void) {
  unsigned v = 0;
  take(&v, 4);
  return v;
}
static int take_i32(void) {
  int v = 0;
  take(&v, 4);
  return v;
}
static double take_f64(void) {
  double v = 0;
  take(&v, 8);
  return v;
}

static void fail(const char *message) {
  g_replyLen = 0;
  put_u8(1);
  put_text(message);
}

/* ---------------------------------------------------------------- loading ----------- */

/* Asks the library for a handle of its own allocator and answers with a pointer into it. */
static void *alloc_handle(Library *lib, unsigned size, unsigned *handle) {
  Request *r = lib->mem;
  r->cmd = CMD_ALLOC;
  r->slot0 = (void *)size;
  r->slot1 = HANDLE_ATTR;
  lib->dispatch(r);
  *handle = (unsigned)r->slot0;
  if (*handle == 0) return NULL;
  r->cmd = CMD_DEREF;
  r->slot0 = (void *)*handle;
  lib->dispatch(r);
  return r->slot0;
}

static void free_handle(Library *lib, unsigned handle) {
  Request *r = lib->mem;
  if (!handle) return;
  r->cmd = CMD_FREE;
  r->slot0 = (void *)handle;
  lib->dispatch(r);
}

static void *deref_handle(Library *lib, unsigned handle) {
  Request *r = lib->mem;
  if (!handle) return NULL;
  r->cmd = CMD_DEREF;
  r->slot0 = (void *)handle;
  lib->dispatch(r);
  return r->slot0;
}

static void unload(Library *lib) {
  if (lib->module) FreeLibrary(lib->module);
  free(lib->req);
  free(lib->mem);
  free(lib->handles);
  free(lib->objects);
  free(lib->funcs);
  memset(lib, 0, sizeof(*lib));
}

/* Walks the FoxTable chain and collects every function the library declares. */
static void collect(Library *lib, FoxTable *table) {
  int room = 0, i;
  FoxTable *t;
  for (t = table; t && room < 4096; t = t->nextLibrary) room += t->infoCount;
  lib->funcs = (FoxInfo **)calloc(room ? room : 1, sizeof(FoxInfo *));
  lib->count = 0;
  for (t = table; t && lib->count < room; t = t->nextLibrary)
    for (i = 0; i < t->infoCount && lib->count < room; i++) lib->funcs[lib->count++] = &t->infoPtr[i];
}

/*
 * Where a library is, spelled the way Windows wants it.
 *
 * A program writes `SET LIBRARY TO c:/tools/thing.fll` as often as it writes a backslash, and
 * LOAD_WITH_ALTERED_SEARCH_PATH only puts the library's own folder on the search path when it
 * can see where that folder is - which it works out of a fully qualified path with backslashes.
 * Given forward slashes it finds no folder and searches only this process's own, which is how a
 * library whose dependencies sit right beside it fails to load.
 */
static void windows_path(const char *path, char *full, size_t size) {
  char native[FLL_PATH];
  size_t i;
  for (i = 0; i + 1 < sizeof(native) && path[i]; i++) native[i] = path[i] == 0x2f ? 0x5c : path[i];
  native[i] = 0;
  if (!GetFullPathNameA(native, (DWORD)size, full, NULL)) {
    strncpy(full, native, size - 1);
    full[size - 1] = 0;
  }
}

/** The folder a path names, without its trailing separator. */
static void folder_of(const char *path, char *out, size_t size) {
  const char *slash = strrchr(path, 0x5c);
  size_t n = slash ? (size_t)(slash - path) : 0;
  if (n >= size) n = size - 1;
  memcpy(out, path, n);
  out[n] = 0;
}

/**
 * Opens the image with `dir` on the search path in front of everything else.
 *
 * This is what makes the ordinary way a Visual FoxPro application ships a library work: the
 * .fll and whatever it needs sit in one folder together, and nothing has to be installed.
 */
static HMODULE open_image(const char *full, const char *dir) {
  HMODULE module;
  SetDllDirectoryA(dir[0] ? dir : NULL);
  module = LoadLibraryExA(full, NULL, LOAD_WITH_ALTERED_SEARCH_PATH);
  SetDllDirectoryA(NULL);
  return module;
}

/*
 * Why a library would not load, in words a person can act on.
 *
 * Windows answers 126 - "the specified module could not be found" - both when the .fll is not
 * there and when something the .fll needs is not there, and never says which. So the file is
 * opened again without resolving anything, its import table is read, and every library it names
 * that cannot be found is named back. vfpencryption71.fll is the case that matters: it needs
 * MSVCP71.dll and MSVCR71.dll, the Visual C++ 7.1 runtimes, which ship with Visual FoxPro and
 * are not part of Windows. Dropping those two beside the .fll is enough, which is worth saying
 * rather than leaving someone with a number.
 */
static void why_not(const char *path, const char *dir, DWORD code, char *error, size_t size) {
  char reason[256];
  char missing[200];
  HMODULE probe;
  size_t n;

  if (!FormatMessageA(FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS, NULL, code,
                      MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT), reason, sizeof(reason), NULL)) {
    sprintf(reason, "Windows error %lu", code);
  }
  n = strlen(reason);
  while (n && (reason[n - 1] == 0x0a || reason[n - 1] == 0x0d || reason[n - 1] == 0x2e || reason[n - 1] == 0x20)) {
    reason[--n] = 0;
  }

  missing[0] = 0;
  SetDllDirectoryA(dir[0] ? dir : NULL);
  probe = LoadLibraryExA(path, NULL, DONT_RESOLVE_DLL_REFERENCES);
  if (probe) {
    __try {
      IMAGE_DOS_HEADER *dos = (IMAGE_DOS_HEADER *)probe;
      IMAGE_NT_HEADERS32 *nt = (IMAGE_NT_HEADERS32 *)((BYTE *)dos + dos->e_lfanew);
      DWORD rva = nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress;
      IMAGE_IMPORT_DESCRIPTOR *imp = (IMAGE_IMPORT_DESCRIPTOR *)((BYTE *)dos + rva);
      for (; rva && imp->Name; imp++) {
        const char *needs = (const char *)((BYTE *)dos + imp->Name);
        HMODULE there = LoadLibraryExA(needs, NULL, LOAD_LIBRARY_AS_DATAFILE);
        if (there) {
          FreeLibrary(there);
          continue;
        }
        if (strlen(missing) + strlen(needs) + 3 < sizeof(missing)) {
          if (missing[0]) strcat(missing, ", ");
          strcat(missing, needs);
        }
      }
    } __except (EXCEPTION_EXECUTE_HANDLER) {
      missing[0] = 0;
    }
    FreeLibrary(probe);
  }
  SetDllDirectoryA(NULL);

  if (missing[0]) {
    _snprintf(error, size, "%.150s needs %.150s, which is not beside it and not anywhere Windows looks", path,
              missing);
  } else if (code == ERROR_MOD_NOT_FOUND) {
    _snprintf(error, size, "%.150s could not be opened: either it is not there, or a library it needs is not beside it",
              path);
  } else {
    _snprintf(error, size, "%.150s: %.200s", path, reason);
  }
  error[size - 1] = 0;
}

static int load_library_at(Library *lib, const char *path, char *error, size_t errorSize) {
  DispatchFn dispatch;
  Request *r;
  FoxTable *table;
  char full[FLL_PATH], dir[FLL_PATH], beside[FLL_PATH];
  DWORD failed;

  memset(lib, 0, sizeof(*lib));
  windows_path(path, full, sizeof(full));
  folder_of(full, dir, sizeof(dir));
  lib->module = open_image(full, dir);
  if (!lib->module) {
    // then the folder this host is running from, for a library that ships with an application
    failed = GetLastError();
    beside[0] = 0;
    if (GetModuleFileNameA(NULL, beside, FLL_PATH)) {
      char self[FLL_PATH];
      folder_of(beside, self, sizeof(self));
      lib->module = open_image(full, self);
    }
    if (!lib->module) {
      why_not(full, dir, failed, error, errorSize);
      return 0;
    }
  }
  dispatch = (DispatchFn)GetProcAddress(lib->module, "@DispatchAPI@4");
  if (!dispatch) {
    FreeLibrary(lib->module);
    lib->module = NULL;
    sprintf(error, "%.200s is not a FoxPro library: it does not export @DispatchAPI@4", full);
    return 0;
  }
  lib->dispatch = dispatch;
  lib->req = (Request *)calloc(1, sizeof(Request));
  lib->mem = (Request *)calloc(1, sizeof(Request));
  lib->handles = (HandleTable *)calloc(1, sizeof(HandleTable));
  lib->objects = (ObjectTable *)calloc(1, sizeof(ObjectTable));
  if (!lib->req || !lib->mem || !lib->handles || !lib->objects) {
    unload(lib);
    strcpy(error, "out of memory");
    return 0;
  }
  /* the first slot of each table is left empty for good: a handle is its index, and index 0
     means "no handle" to the library's own allocator */
  lib->handles->cursor = 1;
  lib->handles->capacity = HANDLE_SLOTS;
  lib->objects->cursor = 1;
  lib->objects->capacity = OBJECT_SLOTS;

  r = lib->req;
  r->cmd = CMD_INIT;
  r->fn = (void *)host_api;
  r->slot0 = lib->handles;
  r->slot1 = (unsigned)lib->objects;
  __try {
    dispatch(r);
  } __except (EXCEPTION_EXECUTE_HANDLER) {
    sprintf(error, "%.200s crashed while being initialised", full);
    unload(lib);
    return 0;
  }
  table = (FoxTable *)r->slot0;
  if (!table) {
    unload(lib);
    sprintf(error, "%.200s answered with no function table", full);
    return 0;
  }
  collect(lib, table);
  /* the file that was actually opened, which is not always the one that was named: a library
     found on the search path, or named without its extension, is still this one */
  if (!GetModuleFileNameA(lib->module, lib->path, FLL_PATH)) strncpy(lib->path, full, FLL_PATH - 1);
  return 1;
}

/* ---------------------------------------------------------------- calling ----------- */

/* Reads one parameter off the wire into a Value, allocating a handle when it is a string. */
static int read_parameter(Library *lib, Value *v, char *error) {
  unsigned char tag = 0;
  memset(v, 0, sizeof(*v));
  if (!take(&tag, 1)) {
    strcpy(error, "a parameter was cut short");
    return 0;
  }
  switch (tag) {
    case 'C': {
      unsigned len = take_u32(), handle = 0;
      void *p;
      if (g_inPos + len > g_inLen) {
        strcpy(error, "a string parameter was cut short");
        return 0;
      }
      p = alloc_handle(lib, len + 1, &handle);
      if (!p) {
        strcpy(error, "the library would not give us memory for a string parameter");
        return 0;
      }
      memcpy(p, g_in + g_inPos, len);
      ((char *)p)[len] = '\0';
      g_inPos += len;
      v->ev_type = 'C';
      v->ev_length = len;
      v->ev_handle = handle;
      break;
    }
    case 'N':
      v->ev_type = 'N';
      v->ev_real = take_f64();
      v->ev_width = (short)take_u16();
      v->ev_length = take_u16();
      break;
    case 'I':
      v->ev_type = 'I';
      v->ev_long = take_i32();
      v->ev_width = (short)take_u16();
      break;
    case 'L':
      v->ev_type = 'L';
      v->ev_length = take_u16();
      break;
    case 'D':
    case 'T':
      v->ev_type = (char)tag;
      v->ev_real = take_f64();
      break;
    case 'Y': {
      v->ev_type = 'Y';
      v->ev_currency.LowPart = take_u32();
      v->ev_currency.HighPart = take_i32();
      break;
    }
    case '0':
      v->ev_type = '0';
      break;
    default:
      sprintf(error, "a parameter of type '%c' cannot be handed to a library", (char)tag);
      return 0;
  }
  return 1;
}

/*
 * What the function wants its n'th parameter to be, out of the FoxInfo's parmTypes string.
 * vfpencryption71.fll declares HASH as "C,.I": the letters are separated by commas and an
 * optional parameter is written with a dot in front of it.
 */
static char wanted_type(const char *types, unsigned index) {
  unsigned at = 0;
  const char *p = types;
  if (!p) return '?';
  for (;;) {
    while (*p == '.' || *p == ' ') p++;
    if (*p == '\0') return '?';
    if (at == index) return *p;
    while (*p && *p != ',') p++;
    if (*p == ',') p++;
    at++;
  }
}

/*
 * Makes a parameter be what the function asked for. This is not a nicety: a library reads the
 * field of the Value its declaration named and nothing else, so an integer parameter arriving
 * as a numeric is read as ev_long, which is zero. Measured: HASH("hello world", 5) answers an
 * empty string when its second parameter is handed over as 'N' and the MD5 digest when it is
 * handed over as 'I', which is what its "C,.I" asked for.
 */
static void coerce(Value *v, char want) {
  switch (want) {
    case 'I':
      if (v->ev_type == 'N') {
        /* A number past the top of a signed long wraps round, as it does in Visual FoxPro: a
           registry root is written 2147483650 for HKEY_LOCAL_MACHINE (0x80000002), and
           CodeMine's cmRegGetValue reads HKLM with it (measured). A plain cast of a double that
           large is undefined, and this compiler makes it 0x80000000 - HKEY_CLASSES_ROOT. */
        if (v->ev_real >= 2147483648.0 && v->ev_real < 4294967296.0) {
          v->ev_long = (long)(unsigned long)v->ev_real;
        } else {
          v->ev_long = (long)v->ev_real;
        }
      } else if (v->ev_type == 'L') {
        v->ev_long = v->ev_length ? 1 : 0;
      } else if (v->ev_type != 'I') {
        return;
      }
      v->ev_type = 'I';
      v->ev_width = 10;
      break;
    case 'N':
      if (v->ev_type == 'I') {
        v->ev_real = (double)v->ev_long;
      } else if (v->ev_type == 'L') {
        v->ev_real = v->ev_length ? 1 : 0;
      } else if (v->ev_type != 'N') {
        return;
      }
      v->ev_type = 'N';
      break;
    case 'L':
      if (v->ev_type == 'I') {
        v->ev_length = v->ev_long ? 1 : 0;
      } else if (v->ev_type == 'N') {
        v->ev_length = v->ev_real != 0.0 ? 1 : 0;
      } else if (v->ev_type != 'L') {
        return;
      }
      v->ev_type = 'L';
      break;
    default:
      break;
  }
}

/* Writes the value a function returned back onto the wire. */
static void write_value(Library *lib, Value *v) {
  (void)lib;
  switch (v->ev_type) {
    /* _RetChar builds its Value with type 'H'; a library that fills one in itself writes 'C'.
       Both mean the same thing: the bytes are in the handle and ev_length says how many. */
    case 'C':
    case 'H':
      /* the bytes were taken while the library still had them; see the _RetVal case above */
      put_u8('C');
      put_bytes32(g_retText ? g_retText : "", g_retTextLen);
      break;
    case 'N':
    case 'B':
      put_u8('N');
      put_f64(v->ev_real);
      put_u16((unsigned short)v->ev_width);
      put_u16(v->ev_length);
      break;
    case 'I':
      put_u8('I');
      put_i32(v->ev_long);
      put_u16((unsigned short)v->ev_width);
      break;
    case 'L':
      put_u8('L');
      put_u8(v->ev_length ? 1 : 0);
      break;
    case 'Y':
      put_u8('Y');
      put_u32(v->ev_currency.LowPart);
      put_i32(v->ev_currency.HighPart);
      break;
    case 'D':
    case 'T':
      put_u8((unsigned)v->ev_type);
      put_f64(v->ev_real);
      break;
    /* what _RetDateStr and _RetDateTimeStr handed over: a string the runtime has to read */
    case 'd':
    case 't': {
      const char *p = (const char *)v->ev_long;
      put_u8(v->ev_type == 'd' ? 'S' : 'W');
      put_bytes32(p ? p : "", p ? strlen(p) : 0);
      break;
    }
    default:
      put_u8('U');
      break;
  }
}

/* Runs one of the library's functions with the parameters already in its request block. */
static int perform(Library *lib, FoxInfo *info, char *error) {
  Request *r = lib->req;
  r->cmd = CMD_CALL;
  r->fn = (void *)info->function;
  __try {
    lib->dispatch(r);
  } __except (EXCEPTION_EXECUTE_HANDLER) {
    sprintf(error, "%.60s crashed", info->funcName ? info->funcName : "the function");
    return 0;
  }
  return 1;
}

/* CALLONLOAD and CALLONUNLOAD are functions the library wants run at those two moments; the
   parmCount field says so in place of a count. */
static void run_hooks(Library *lib, short which) {
  char error[256];
  int i;
  for (i = 0; i < lib->count; i++) {
    if (lib->funcs[i]->parmCount != which) continue;
    lib->req->pCount = 0;
    perform(lib, lib->funcs[i], error);
  }
}

/* ---------------------------------------------------------------- the loop ---------- */

static void reset_call(void) {
  g_outLen = 0;
  g_hasRet = 0;
  g_retTextLen = 0;
  g_errNo = 0;
  g_errText[0] = '\0';
  g_missing[0] = '\0';
  memset(&g_ret, 0, sizeof(g_ret));
}

static void do_load(void) {
  char path[FLL_PATH];
  char error[512];
  unsigned len = take_u16();
  int slot, i;
  Library *lib;

  if (len >= FLL_PATH || g_inPos + len > g_inLen) {
    fail("the path was too long");
    return;
  }
  memcpy(path, g_in + g_inPos, len);
  path[len] = '\0';
  g_inPos += len;

  for (slot = 0; slot < MAX_LIBRARIES; slot++)
    if (!g_lib[slot].module) break;
  if (slot == MAX_LIBRARIES) {
    fail("too many libraries are loaded");
    return;
  }
  lib = &g_lib[slot];
  if (!load_library_at(lib, path, error, sizeof(error))) {
    fail(error);
    return;
  }
  reset_call();
  g_current = lib;
  run_hooks(lib, CALLONLOAD);
  g_current = NULL;

  put_u8(0);
  put_u16(slot);
  put_text(lib->path);
  put_u16(lib->count);
  for (i = 0; i < lib->count; i++) {
    FoxInfo *f = lib->funcs[i];
    put_text(f->funcName);
    put_u16((unsigned short)f->parmCount);
    put_text(f->parmTypes);
  }
  put_bytes32(g_out, g_outLen);
}

static void do_call(void) {
  char error[512];
  unsigned slot = take_u16();
  unsigned index = take_u16();
  unsigned argc = take_u16();
  Library *lib;
  Request *r;
  unsigned i;

  if (slot >= MAX_LIBRARIES || !g_lib[slot].module) {
    fail("that library is not loaded");
    return;
  }
  lib = &g_lib[slot];
  if (index >= (unsigned)lib->count) {
    fail("that library has no such function");
    return;
  }
  if (argc > MAX_PARMS) {
    fail("too many arguments");
    return;
  }
  reset_call();
  r = lib->req;
  memset(&r->pCount, 0, sizeof(short) + sizeof(FoxParameter) * MAX_PARMS);
  r->pCount = (short)argc;
  for (i = 0; i < argc; i++) {
    g_refIs[i] = g_refStored[i] = 0;
    /* 'R' is a variable passed by reference: its value follows, and the library gets a Locator */
    if (g_inPos < g_inLen && g_in[g_inPos] == 'R') {
      g_inPos++;
      if (!read_parameter(lib, &g_refs[i], error)) {
        fail(error);
        return;
      }
      if (g_refs[i].ev_type == 'C') {
        ref_keep_text(i, (const char *)deref_handle(lib, g_refs[i].ev_handle), g_refs[i].ev_length);
        free_handle(lib, g_refs[i].ev_handle);
        g_refs[i].ev_handle = 0;
      }
      g_refIs[i] = 1;
      memset(&r->p[i], 0, sizeof(FoxParameter));
      r->p[i].loc.l_type = 'R';
      r->p[i].loc.l_where = -1;
      r->p[i].loc.l_NTI = (USHORT)(i + 1);
      continue;
    }
    if (!read_parameter(lib, &r->p[i].val, error)) {
      for (; i > 0; i--)
        if (r->p[i - 1].val.ev_type == 'C') free_handle(lib, r->p[i - 1].val.ev_handle);
      fail(error);
      return;
    }
    coerce(&r->p[i].val, wanted_type(lib->funcs[index]->parmTypes, i));
    /* a value where the function declared a reference is refused before it runs: the library
       would read a Locator out of it. Measured: error 9, "Data type mismatch". */
    if (wanted_type(lib->funcs[index]->parmTypes, i) == 'R') g_errNo = 9;
  }

  g_current = lib;
  if (g_errNo == 0 && !perform(lib, lib->funcs[index], error)) {
    g_current = NULL;
    fail(error);
    return;
  }

  /* what the library was given is the host's to let go of; what it handed back was taken
     while the library still had it */
  for (i = 0; i < argc; i++)
    if (!g_refIs[i] && r->p[i].val.ev_type == 'C') free_handle(lib, r->p[i].val.ev_handle);

  put_u8(0);
  if (g_hasRet) {
    write_value(lib, &g_ret);
  } else {
    put_u8('U');
  }
  g_current = NULL;
  put_bytes32(g_out, g_outLen);
  put_i32(g_errNo);
  put_text(g_errText);
  put_text(g_missing);
  /* what the library stored in the variables it was handed by reference */
  {
    unsigned stored = 0;
    for (i = 0; i < argc; i++) stored += g_refIs[i] && g_refStored[i];
    put_u16(stored);
    for (i = 0; i < argc; i++) {
      if (!(g_refIs[i] && g_refStored[i])) continue;
      put_u16(i);
      if (g_refs[i].ev_type == 'C') {
        put_u8('C');
        put_bytes32(g_refText[i] ? g_refText[i] : "", g_refTextLen[i]);
      } else {
        write_value(lib, &g_refs[i]);
      }
    }
  }
}

static void do_unload(void) {
  unsigned slot = take_u16();
  if (slot < MAX_LIBRARIES && g_lib[slot].module) {
    reset_call();
    g_current = &g_lib[slot];
    run_hooks(&g_lib[slot], CALLONUNLOAD);
    g_current = NULL;
    unload(&g_lib[slot]);
  }
  put_u8(0);
}

int main(int argc, char **argv) {
  unsigned length;
  _setmode(_fileno(stdin), _O_BINARY);
  _setmode(_fileno(stdout), _O_BINARY);
  if (argc > 1) {
    g_pipe = CreateNamedPipeA(argv[1], PIPE_ACCESS_DUPLEX,
                              PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT, 1, 1 << 16, 1 << 16, 0, NULL);
    if (g_pipe == INVALID_HANDLE_VALUE) return 2;
    /* the caller may already have opened its end while the pipe was being made, which Windows
       reports as a failure to connect with ERROR_PIPE_CONNECTED */
    if (!ConnectNamedPipe(g_pipe, NULL) && GetLastError() != ERROR_PIPE_CONNECTED) return 2;
  }
  /* a library that puts up a message box would wedge a process nobody can see */
  SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);

  for (;;) {
    unsigned char op = 0;
    if (!read_exact(&length, 4)) break;
    if (length > 64u * 1024u * 1024u) break;
    g_in = (unsigned char *)realloc(g_in, length ? length : 1);
    if (!g_in) break;
    if (!read_exact(g_in, length)) break;
    g_inLen = length;
    g_inPos = 0;
    g_replyLen = 0;
    take(&op, 1);
    switch (op) {
      case 1:
        do_load();
        break;
      case 2:
        do_call();
        break;
      case 3:
        do_unload();
        break;
      case 4:
        put_u8(0);
        break;
      default:
        fail("that is not something this host does");
        break;
    }
    length = (unsigned)g_replyLen;
    write_all(&length, 4);
    write_all(g_reply, g_replyLen);
  }
  if (g_pipe != INVALID_HANDLE_VALUE) CloseHandle(g_pipe);
  return 0;
}
