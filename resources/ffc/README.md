# Visual FoxPro Foundation Classes

The `.vcx` / `.vct` class libraries a Visual FoxPro project builds its forms from, bundled here
so that importing a project resolves the classes it uses.

A form that uses a Foundation Class records only the library's file name (`CLASSLOC` is
`_base.vcx`, with no directory), because Visual FoxPro finds these on its own search path.
Without them, `_commandbutton`, `_hyperlinklabel`, `_navbtns` and the rest cannot be resolved,
and every control stamped from one imports as an empty base control.

They are read, never executed: the importer parses them the same way it parses any other `.vcx`,
to learn what a class contains. The headers, the bitmaps beside them and `openadlg.scx` come
with the libraries because the classes name them.

## Where they came from

This is the VFPX release of the Foundation Classes, from <https://github.com/VFPX/FFC>, which
Microsoft published under the Microsoft Permissive License. `FFC_EULA.txt` beside them is that
licence, and it governs these files rather than the licence in the repository root.

## `foxpro.h`, which is not one of them either

`wincrypt.h` beside `_crypt.vcx` opens with `#INCLUDE foxpro.h`, and so do several of the other
headers here. That file is Visual FoxPro's own list of constants (`T_CHARACTER`, the message box
flags, the field types); it sits in the root of a Visual FoxPro installation rather than in the
Foundation Classes, and the product finds it on its own search path. Without it every class whose
header includes it compiles with those names unknown, and the first method that uses one refuses.

Like `builderd.vcx` below, it comes from a Visual FoxPro installation and is governed by that
product's licence rather than by the Ms-PL beside it.

## `builderd.vcx`, which is not one of them

`_ws3utils.vcx` builds `wsbasebuilder` from `builderbaseform` in `..\wizards\builderd.vcx`: a
file from Visual FoxPro's own Wizards folder, not from the Foundation Classes. It is here so
that class resolves rather than importing as a plain form with its members missing.

It is the one file in this folder that the Ms-PL beside it does not cover: it comes from a
Visual FoxPro installation and is governed by that product's licence. Anyone redistributing this
build should satisfy themselves that they may pass it on.
