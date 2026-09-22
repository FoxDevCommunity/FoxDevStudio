import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { BASE_CLASS_MEMBERS } from '@shared/registry';
// the generator is a plain script so that it can be run without the TypeScript pipeline; this
// asks it what the two files should say and compares that with what they do say
import { generate } from '../../scripts/gen-base-classes.mjs';

/**
 * The object model is measured, and these are the files that carry the measurement.
 *
 * `tests/reference/vfp-base-classes.tsv` is what vfp9.exe answered (see
 * `scripts/vfp-base-classes.mjs`). `src/shared/registry/baseClassMembers.ts` and
 * `crates/foxvm/src/base_classes.tsv` are generated from it, one for the running application and
 * one for the VM's own test host, so that both build the same object. Editing either by hand
 * would put the runtime back to guessing; this is what stops that.
 *
 * Regenerate with `node scripts/gen-base-classes.mjs`.
 */
describe('the base classes', () => {
  it('are generated from the measurement, and both copies are current', () => {
    for (const [path, want] of Object.entries(generate())) {
      const had = readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
      expect(had, `${path} is stale; rerun node scripts/gen-base-classes.mjs`).toBe(want);
    }
  });

  it('cover every class a form is made of, and say what a new one holds', () => {
    const wanted = [
      'Form', 'Label', 'TextBox', 'EditBox', 'CommandButton', 'CommandGroup', 'CheckBox',
      'OptionGroup', 'OptionButton', 'ComboBox', 'ListBox', 'Spinner', 'Container', 'PageFrame',
      'Page', 'Grid', 'Column', 'Header', 'Image', 'Line', 'Shape', 'Timer', 'Custom', 'Session',
      'Collection', 'Exception', 'Relation', 'Cursor', 'DataEnvironment',
    ];
    for (const name of wanted) expect(BASE_CLASS_MEMBERS[name], name).toBeDefined();
    // the shape of the thing, spot-checked against what the product answered
    expect(BASE_CLASS_MEMBERS['CommandButton']!.properties['Caption']).toBe('Command');
    expect(BASE_CLASS_MEMBERS['CommandButton']!.properties['Height']).toBe(17);
    expect(BASE_CLASS_MEMBERS['Label']!.properties['BackColor']).toBe(0xffffff);
    expect(BASE_CLASS_MEMBERS['Form']!.readOnly['BaseClass']).toBeGreaterThan(0);
    expect(BASE_CLASS_MEMBERS['Label']!.methods).not.toContain('AddObject');
    expect(BASE_CLASS_MEMBERS['Form']!.methods).toContain('AddObject');
    // an Empty answers to nothing at all until a program puts something on it
    expect(Object.keys(BASE_CLASS_MEMBERS['Empty']!.properties)).toEqual([]);
    expect(BASE_CLASS_MEMBERS['Empty']!.methods).toEqual([]);
  });
});
