import { describe, expect, it } from 'vitest';
import { allNodes, cloneSubtree, findByName, findLocation, findNode, insertChild, isDescendantOf, removeNode, reorder, reparent } from '@shared/form/tree';
import { sampleForm } from '../helpers/fixtures';

describe('form tree', () => {
  it('finds nodes at any depth with their location', () => {
    const { form } = sampleForm();
    expect(allNodes(form).map((n) => n.id)).toEqual(['Command1', 'Command2', 'Text1', 'Pageframe1', 'Page1', 'Label1', 'Page2', 'Timer1']);
    const loc = findLocation(form, 'Label1')!;
    expect(loc.parent).toBe(findNode(form, 'Page1'));
    expect(loc.index).toBe(0);
    expect(loc.ancestors.map((a) => a.id)).toEqual(['Pageframe1', 'Page1']);
    expect(findByName(form, 'LABEL1')?.id).toBe('Label1');
    expect(isDescendantOf(form, 'Label1', 'Pageframe1')).toBe(true);
    expect(isDescendantOf(form, 'Command1', 'Pageframe1')).toBe(false);
  });

  it('inserts, removes, reparents and refuses cycles', () => {
    const { form } = sampleForm();
    const removed = removeNode(form, 'Command2')!;
    expect(removed.name).toBe('Command2');
    expect(findNode(form, 'Command2')).toBeUndefined();
    insertChild(findNode(form, 'Page2')!, removed, 0);
    expect(findLocation(form, 'Command2')!.parent).toBe(findNode(form, 'Page2'));

    expect(reparent(form, 'Label1', null, 0)).toBe(true);
    expect(form.children[0]!.id).toBe('Label1');
    expect(reparent(form, 'Pageframe1', 'Page1')).toBe(false); // pageframe into its own page
    expect(findNode(form, 'Pageframe1')).toBeDefined();
  });

  it('changes z-order within a parent', () => {
    const { form } = sampleForm();
    const ids = () => form.children.map((c) => c.id);
    reorder(form, 'Command1', 'front');
    expect(ids()[ids().length - 1]).toBe('Command1');
    reorder(form, 'Command1', 'back');
    expect(ids()[0]).toBe('Command1');
    reorder(form, 'Command1', 'forward');
    expect(ids()[1]).toBe('Command1');
    reorder(form, 'Command1', 'backward');
    expect(ids()[0]).toBe('Command1');
  });

  it('clones subtrees with fresh ids', () => {
    const { form } = sampleForm();
    let n = 0;
    const clone = cloneSubtree(findNode(form, 'Pageframe1')!, () => `new${++n}`);
    expect(clone.id).toBe('new1');
    expect(clone.children![0]!.id).toBe('new2');
    expect(clone.children![0]!.children![0]!.id).toBe('new3');
    expect(clone.children![0]!.children![0]!.props).not.toBe(findNode(form, 'Label1')!.props);
  });
});
