# What this runtime will not do

Everything else this project has not finished is a gap: it is on a list, it has a reason, and it
is going to be built. This file is the other thing. What is written here is refused on purpose,
and asking again will not change the answer.

The distinction matters because the two look identical from the Output window. A developer who
sees "Class definition is not found" cannot tell "nobody has written this yet" from "this will
never work, and here is why". So every entry below is refused **by name, with its reason**, and
the runtime says so rather than answering with the generic error.

Nothing here stops a form opening. The form loads, the rest of it works, and only the control
that asks for one of these refuses.

## Withdrawn technology

The product it needs no longer exists. There is nothing to measure against, on this machine or
any other, so the behaviour could never be verified even in principle. That is the test for this
section: not "old", but "gone".

### Microsoft Agent

`Agent.Control.2`, and the character files `Genie.acs`, `Merlin.acs`, `Peedy.acs` and `Robby.acs`
that go with it.

The animated desktop assistant. Microsoft removed it from Windows after Vista and shipped no
replacement. The control cannot be installed on a supported version of Windows, so a form built
around it cannot be made to work by any amount of effort here.

Reached by `Solution/Ffc/agent.scx` in the Visual FoxPro samples.

### Windows Messenger automation

`Messenger.UIAutomation.1`.

Automation over the Windows Messenger client. The service behind it was shut down in 2013 and the
client was removed from Windows. Even with the control present there is no network on the other
side of it.

Reached by `Solution/Toledo/messenger.scx`, which already expects to fail and handles the error
itself.

## Controls that will not load here

Refused for a different and weaker reason, and the difference is recorded rather than hidden:
the control may well still load on a machine that has it. What is missing is our ability to
**measure** the behaviour, and this project's rule is that unmeasured behaviour does not get
written.

If one becomes measurable, the entry is revisited rather than defended. That has happened once
already, and what it taught is worth more than the entry was.

### The Multimedia MCI control

`mci32.ocx`, which `Solution/Ole/mmsample.scx` embeds and reads `hwndDisplay` from. It is
registered only as a 32-bit in-process server, so nothing in a 64-bit process can create it.

### Taken out: the Windows Media Player and its cdromCollection

This section used to carry a second entry. It said `Solution/Europa/foxmedia.scx` could not work
because the machine has no CD-ROM drive and the legacy ActiveX player is no longer installed by
default.

It was measured, and it is false. `WMPlayer.OCX` creates from a 64-bit process on this machine,
its `cdromCollection` answers, `Count` is 0 because there is no drive, the form's
`FOR i = 1 TO lnCount` therefore does not run, the combo box stays empty and the form opens
without a word. Having no drive is exactly what the collection is for reporting; it was never
what stopped the code. What actually stopped it was ours: `AddObject` was dropping the third
argument, the OLE class, so `AddObject("ole1", "olecontrol", "WMPlayer.OCX")` built a control
that had no class to ask COM for.

`Solution/Europa/foxmedia.scx` is off the samples baseline, and the entry is gone. The note
stays because the mistake is the useful part: an entry written from what a control was assumed
to be, rather than from what it did, is not evidence of anything. Nothing goes in this file that
has not been tried.

## Not refused: 32-bit COM in a 64-bit process

This section is the one thing in this file that is **not** a refusal. It is a deferral with a
known answer, and it accounts for more of the samples baseline than everything above it put
together. It is written down here because from the Output window it looks exactly like the
refusals, and it is not one.

Measured on this machine. `mscomctl.ocx` (the Common Controls: ListView, TreeView, ImageList),
`richtx32.ocx` (the Rich Text control), `mci32.ocx` (the Multimedia MCI control), `shdocvw.dll`
(the WebBrowser control) and MSChart each register their `InProcServer32` **only** under
`HKLM\SOFTWARE\Classes\Wow6432Node\CLSID\{...}`, pointing at a file in `SysWOW64`. The 64-bit
view of the same CLSID has the class but no server under it. So `CoCreateInstance` from a
64-bit process answers `REGDB_E_CLASSNOTREG` - "class not registered" - for a control that is
installed and working.

That answer is indistinguishable from the one a class nobody has installed gives, which is why
this was read for a long time as software the machine did not have. It is not. vfp9.exe is a
32-bit process, and on this machine it opens every one of these forms without saying anything:

- `Solution/Europa/WindowsEvents.scx` (WebBrowser)
- `Solution/Ole/mmsample.scx` (MCI)
- `Solution/Ole/olegraph.scx` (MSChart)
- `Solution/Ole/rtf.scx` (Rich Text)
- `Solution/Tahoe/querydd.scx` (ListView, TreeView, WebBrowser, Rich Text)
- `Solution/Toledo/xmladapter.scx` (WebBrowser)

None of those forms fails because of anything missing in this runtime. They fail because of the
width of the process.

### The answer already exists in this repository

This is the same wall `SET LIBRARY TO` hit. An FLL is a 32-bit DLL, the Electron process is
64-bit, and it cannot load one either. The way round it was `fllhost.exe`: a small 32-bit helper
process that loads the library and answers over a pipe, with the 64-bit side holding nothing but
a handle.

A 32-bit COM surrogate in that same shape - the helper creating the object and forwarding
property reads, property writes and method calls to it - would make every control in this
section work, and would take `mci32.ocx` out of the section above as well. The napi addon this
runtime talks COM through already has exactly that interface, so what is needed is the process
boundary rather than a new design. It is a known amount of work on a road already built.

Until it exists, the controls in this section answer the way an unregistered class does, and
their lines stay on the baseline.

## What this means for the samples baseline

`tests/vfp/samples-run-known.txt` records what each of the 123 shipped sample forms says while it
opens, and it only ever shrinks. Three of its lines are refusals - Microsoft Agent, Windows
Messenger and the MCI control - and those three are not going to come off. They are marked here
so that nobody reads the remaining count as work still to do.

The forms in the 32-bit COM section are the opposite: their lines are work still to do, with the
work already named. They come off the day the surrogate exists.

Counting the list against this file is the honest way to read it: the score is the lines that
are **not** refused here.
