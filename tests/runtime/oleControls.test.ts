/**
 * The Common Controls FoxDev provides in place of the ActiveX ones, driven the way a form
 * drives them: through the desktop's host reads, by handle, exactly as the VM would.
 */

import { describe, expect, it } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import { ImageList, TreeView } from '@shared/runtime/oleObjects';
import type { FormNode } from '@shared/form/schema';
import type { VmValue } from '@shared/runtime/values';

/** A form with a TreeView and an ImageList on it, as an imported `.scx` produces. */
function form(): FormNode {
  return {
    name: 'frmTree',
    props: {},
    methods: {},
    children: [
      { id: 'a', type: 'OleControl', name: 'oleTree', props: { OleClass: 'MSComctlLib.TreeCtrl.2' }, methods: {} },
      { id: 'b', type: 'OleControl', name: 'oleImages', props: { OleClass: 'MSComctlLib.ImageListCtrl.2' }, methods: {} },
      { id: 'c', type: 'OleControl', name: 'oleOther', props: { OleClass: '(sysinfo.ocx)' }, methods: {} },
    ],
  };
}

function open(): { desktop: Desktop; tree: number; images: number; other: number } {
  const desktop = new Desktop();
  const instance = desktop.instantiate(form(), -1);
  const child = (name: string) => instance.child(name)!.handle;
  return { desktop, tree: child('oleTree'), images: child('oleImages'), other: child('oleOther') };
}

/** `obj.name` as the VM reads it: a member that is an object comes back as a handle. */
const handle = (desktop: Desktop, obj: number, name: string): number => {
  const member = desktop.getMember(obj, name);
  if (typeof member !== 'number') throw new Error(`${name} is ${member}, not an object`);
  return member;
};

describe('emulated ActiveX controls', () => {
  it('gives a TreeView to a control whose class is one', () => {
    const { desktop, tree, images, other } = open();
    expect(desktop.object(tree)?.ole).toBeInstanceOf(TreeView);
    expect(desktop.object(images)?.ole).toBeInstanceOf(ImageList);
    expect(desktop.object(other)?.ole).toBeNull();
  });

  it('adds nodes the way filltree does, and reads them back', () => {
    const { desktop, tree } = open();
    const nodes = handle(desktop, tree, 'Nodes');

    // oNode = o.Nodes.Add(, 1, "root", "Samples")
    const root = desktop.callMethod(nodes, 'Add', [false, 1, 'root', 'Samples']) as { $obj: number };
    // o.Nodes.Add("root", 4, "leaf", "Forms")
    desktop.callMethod(nodes, 'Add', ['root', 4, 'leaf', 'Forms']);

    expect(desktop.getProp(nodes, 'Count')).toBe(2);
    expect(desktop.getProp(root.$obj, 'Text')).toBe('Samples');
    expect(desktop.getProp(root.$obj, 'Children')).toBe(1);
    expect(desktop.getProp(root.$obj, 'Index')).toBe(1);

    // oNode.Image = "leaf"
    desktop.setProp(root.$obj, 'Image', 'leaf');
    expect(desktop.getProp(root.$obj, 'Image')).toBe('leaf');

    // THISFORM.oleTree.Nodes(2).Expanded = .T.
    const second = desktop.callMethod(nodes, 'Item', [2]) as { $obj: number };
    expect(desktop.getProp(second.$obj, 'Text')).toBe('Forms');
    expect(desktop.getProp(second.$obj, 'FullPath')).toBe('Samples\\Forms');
  });

  it('draws the nodes that are not inside a collapsed one', () => {
    const { desktop, tree } = open();
    const nodes = handle(desktop, tree, 'Nodes');
    desktop.callMethod(nodes, 'Add', [false, 1, 'a', 'A']);
    desktop.callMethod(nodes, 'Add', ['a', 4, 'a1', 'A1']);
    const control = desktop.object(tree)!.ole as TreeView;

    expect(control.visible().map((v) => v.node.text)).toEqual(['A']);
    desktop.setProp((desktop.callMethod(nodes, 'Item', [1]) as { $obj: number }).$obj, 'Expanded', true);
    expect(control.visible().map((v) => v.node.text)).toEqual(['A', 'A1']);
  });

  it('takes an ImageList by reference, as `o.ImageList = THIS.oleImages` does', () => {
    const { desktop, tree, images } = open();
    desktop.setProp(tree, 'ImageList', { $obj: images } as VmValue);
    expect(desktop.object(tree)!.ole).toBeInstanceOf(TreeView);
    expect((desktop.object(tree)!.ole as TreeView).imageList).toBe(desktop.object(images)!.ole);

    const list = handle(desktop, images, 'ListImages');
    desktop.callMethod(list, 'Add', [1, 'leaf', 'leaf.bmp']);
    expect(desktop.getProp(list, 'Count')).toBe(1);
    expect((desktop.object(images)!.ole as ImageList).picture('leaf')).toBe('leaf.bmp');
  });

  it('sorts when asked, and keeps the control own properties for itself', () => {
    const { desktop, tree } = open();
    const nodes = handle(desktop, tree, 'Nodes');
    desktop.callMethod(nodes, 'Add', [false, 1, 'b', 'Beta']);
    desktop.callMethod(nodes, 'Add', [false, 1, 'a', 'Alpha']);
    desktop.setProp(tree, 'Sorted', true);
    expect((desktop.object(tree)!.ole as TreeView).roots.map((n) => n.text)).toEqual(['Alpha', 'Beta']);

    // Visible belongs to the form's control, not to the TreeView behind it
    desktop.setProp(tree, 'Visible', false);
    expect(desktop.object(tree)?.get('Visible')).toBe(false);
  });

  it('still refuses a control it does not provide', () => {
    const { desktop, other } = open();
    expect(() => desktop.callMethod(other, 'Whatever', [])).toThrow(/ActiveX/);
  });
});
