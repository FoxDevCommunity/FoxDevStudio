/**
 * Working out which ActiveX control a Visual FoxPro `.scx` row holds.
 *
 * An OLE control's row keeps two memos. `OLE2` is a line of text naming the file the control came
 * from - `OLEObject = C:\WINNT\System32\MSCOMCTL.OCX` - which says which OCX but not which of the
 * eight controls inside it. `OLE` is the control's persisted state as an OLE compound file, and
 * the class id is in there: not in the root storage's own CLSID, which VFP leaves blank, but
 * inside the `OleObjectData` stream it writes.
 *
 * Rather than walk the compound file's allocation tables to reach that stream, the blob is
 * scanned for a GUID this module recognises. That cannot misidentify anything: a control is only
 * named when one of its known class ids is found, and everything else stays an unknown ActiveX
 * control.
 *
 * The ids come in pairs. Every Common Control was re-issued with a new class id when Microsoft
 * shipped `mscomctl.ocx` version 6, and forms built against either version are still in the
 * wild - the samples that ship with Visual FoxPro 9 use the older ones throughout.
 */

/** A control this runtime knows by name, whether or not it can also draw one. */
export interface OleControlClass {
  /** The ProgID, as VFP would show it in the property sheet. */
  progId: string;
  /** What the runtime provides for it, when it provides anything. */
  emulated?: 'TreeView' | 'ImageList';
}

const CLASSES: Record<string, OleControlClass> = {
  // TreeView: the original mscomctl.ocx, then the version 6 re-issue
  '{C741FDB6-2630-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.TreeCtrl.2', emulated: 'TreeView' },
  '{C74190B6-8589-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.TreeCtrl.2', emulated: 'TreeView' },
  '{2C247F23-2618-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.ImageListCtrl.2', emulated: 'ImageList' },
  '{2C247F23-8591-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.ImageListCtrl.2', emulated: 'ImageList' },
  // named but not emulated: enough for a program to be told what it is asking for
  '{BDD1F04B-2639-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.ListViewCtrl.2' },
  '{BDD1F04B-858B-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.ListViewCtrl.2' },
  '{F0FDF954-2619-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.Slider.2' },
  '{F08DF954-8592-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.Slider.2' },
  '{7D3867A3-2620-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.SBarCtrl.2' },
  '{8E3867A3-8586-11D1-B16A-00C0F0283628}': { progId: 'MSComctlLib.SBarCtrl.2' },
  '{3B7CC660-D7FD-101B-B9B5-04021C001D02}': { progId: 'RichText.RichTextCtrl.1' },
  '{F9043C26-F6F2-101A-A3C9-08002B2F49FB}': { progId: 'MSComDlg.CommonDialog.1' },
  '{C656F961-340A-11D0-A96B-00C04FD705A2}': { progId: 'Shell.Explorer.2' },
  '{20C62CAB-15DA-101B-B9A8-444553540000}': { progId: 'MSMAPI.MAPIMessages' },
  '{20C62CA0-15DA-101B-B9A8-444553540000}': { progId: 'MSMAPI.MAPISession' },
};

/** Formats the 16 bytes at `at` as a registry-style GUID. */
function guidAt(blob: Uint8Array, at: number): string {
  const byte = (i: number) => (blob[i] ?? 0).toString(16).padStart(2, '0');
  const little = (from: number, len: number) => Array.from({ length: len }, (_, k) => byte(from + len - 1 - k)).join('');
  const big = (from: number, len: number) => Array.from({ length: len }, (_, k) => byte(from + k)).join('');
  return `{${little(at, 4)}-${little(at + 4, 2)}-${little(at + 6, 2)}-${big(at + 8, 2)}-${big(at + 10, 6)}}`.toUpperCase();
}

/**
 * The class id of the control persisted in an `OLE` memo, if it is one this module knows.
 *
 * The memo crosses the wasm bridge as bytes, because no code page would survive it.
 */
export function oleClassId(blob: Uint8Array): string | null {
  for (let i = 0; i + 16 <= blob.length; i++) {
    // the first four bytes settle it for all but a handful of offsets, and cost no string
    const data1 = (blob[i]! | (blob[i + 1]! << 8) | (blob[i + 2]! << 16)) + blob[i + 3]! * 0x1000000;
    if (!FIRST_WORDS.has(data1)) continue;
    const candidate = guidAt(blob, i);
    if (candidate in CLASSES) return candidate;
  }
  return null;
}

/** The leading 32 bits of every known class id, for rejecting an offset without building one. */
const FIRST_WORDS = new Set(Object.keys(CLASSES).map((id) => Number.parseInt(id.slice(1, 9), 16)));

/** What a class id is, by name. */
export function oleControlClass(classId: string): OleControlClass | null {
  return CLASSES[classId.toUpperCase()] ?? null;
}

/**
 * What this runtime provides for a control, named either by class id or by the ProgID an
 * imported form recorded in `OleClass`.
 */
export function oleEmulation(name: string): OleControlClass['emulated'] | null {
  const wanted = name.trim().toLowerCase();
  if (wanted === '') return null;
  for (const [id, entry] of Object.entries(CLASSES)) {
    if (!entry.emulated) continue;
    if (wanted === id.toLowerCase() || wanted === entry.progId.toLowerCase()) return entry.emulated;
  }
  return null;
}

/** The file an `OLE2` memo names, without its path: `mscomctl.ocx`. */
export function oleServerFile(memo: string): string {
  const match = /OLEObject\s*=\s*(.+)/i.exec(memo);
  if (!match) return '';
  return (match[1] ?? '').trim().replace(/^.*[\\/]/, '');
}

/**
 * What to record as an OLE control's `OleClass`: its ProgID when the class is known, else the
 * file it came from, so the property sheet and any error message can still say what it is.
 */
export function oleClassOf(oleMemo: Uint8Array, ole2Memo: string): string {
  const id = oleClassId(oleMemo);
  if (id) return CLASSES[id]!.progId;
  const file = oleServerFile(ole2Memo);
  return file === '' ? '' : `(${file})`;
}
