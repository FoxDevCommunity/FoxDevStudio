# The 32-bit library host

`SET LIBRARY TO something.fll` loads a Visual FoxPro library. Every .fll ever shipped is a
32-bit Win32 image and every process this application runs is 64-bit, so none of them can load
one: a 64-bit process cannot map a 32-bit image, and no amount of FFI changes that. `fllhost.exe`
is a 32-bit program of its own that does the loading and is spoken to over a named pipe. Its
source is `native/fllhost/fllhost.c` and `scripts/build-fllhost.mjs` builds it; nothing at run
time compiles anything.

It links the C runtime statically, so it needs no Visual C++ redistributable of its own. It is
here rather than beside `foxole.node` because the two files below belong next to it.

## `msvcr71.dll` and `msvcp71.dll`

Microsoft's Visual C++ 7.1 runtime (the C and C++ libraries of Visual Studio .NET 2003)
redistributed with this application.

They are here for the libraries, not for the host. A .fll built with that toolset links against
these two by name, and almost every .fll still in use was: `vfpencryption71.fll`, the encryption
library most Visual FoxPro applications reach for, imports both. They ship with Visual FoxPro 9
and are not part of Windows, so on a machine that has never had Visual FoxPro installed such a
library cannot load unless they are somewhere the loader will look.

Beside the host is exactly that place. Windows searches the directory of the process doing the
loading, and for a .fll that process is `fllhost.exe`, not Electron, so a library finds its
runtime here wherever the library itself came from, with nothing installed and no PATH changed.
A library whose own dependencies sit beside it works too: the host puts the library's folder on
the search path before opening it.

The copies here are the 32-bit ones, taken from `%SystemRoot%\SysWOW64`, which is where 64-bit
Windows keeps the 32-bit system libraries. The ones in `System32` on a 64-bit machine are
64-bit and would be no use to a 32-bit library.

`scripts/build-fllhost.mjs` copies them when they are not already here; a machine without them
builds a host that works and says which library was missing when a .fll that needs them is
loaded. The two files are tracked in git, unlike everything else in this folder, because the
build runners that make the nightlies have no Visual FoxPro to take them from.

These two files are Microsoft's and are governed by that company's redistribution terms rather
than by the licence in the repository root. Anyone redistributing this build should satisfy
themselves that they may pass them on.
