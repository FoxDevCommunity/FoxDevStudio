import type { FormDocument } from '@shared/form/schema';
import type { MenuDocument } from '@shared/menu/schema';
import type { ProjectDocument } from '@shared/project/schema';

/**
 * A small form with two buttons, a text box and a page frame; ids equal names for readability.
 *
 * Sizes are written out as a document the designer made carries them: a control gets the size
 * the designer drops it at, which is not the size the class itself starts out.
 */
export function sampleForm(): FormDocument {
  return {
    $schema: 'foxdev-form',
    version: 1,
    form: {
      name: 'Form1',
      props: { Caption: 'Sample', Width: 400, Height: 300 },
      methods: { Init: '' },
      children: [
        { id: 'Command1', type: 'CommandButton', name: 'Command1', props: { Left: 16, Top: 16, Width: 84, Height: 27, Caption: 'OK' }, methods: { Click: 'WAIT WINDOW "ok"' } },
        { id: 'Command2', type: 'CommandButton', name: 'Command2', props: { Left: 120, Top: 16, Width: 84, Height: 27, Caption: 'Cancel' }, methods: {} },
        { id: 'Text1', type: 'TextBox', name: 'Text1', props: { Left: 16, Top: 64, Width: 200, Height: 23, Value: 'hello' }, methods: {} },
        {
          id: 'Pageframe1',
          type: 'PageFrame',
          name: 'Pageframe1',
          props: { Left: 16, Top: 100, Width: 300, Height: 150 },
          methods: {},
          children: [
            { id: 'Page1', type: 'Page', name: 'Page1', props: { Caption: 'First' }, methods: {}, children: [
              { id: 'Label1', type: 'Label', name: 'Label1', props: { Left: 8, Top: 8, Caption: 'Inside' }, methods: {} },
            ] },
            { id: 'Page2', type: 'Page', name: 'Page2', props: { Caption: 'Second' }, methods: {}, children: [] },
          ],
        },
        { id: 'Timer1', type: 'Timer', name: 'Timer1', props: { Left: 300, Top: 260, Interval: 1000 }, methods: { Timer: '' } },
      ],
    },
  };
}

export function sampleMenu(): MenuDocument {
  return {
    $schema: 'foxdev-menu',
    version: 1,
    name: 'Main',
    location: 'Replace',
    items: [
      {
        id: 'file',
        prompt: '\\<File',
        result: { type: 'submenu' },
        children: [
          { id: 'new', prompt: '\\<New', result: { type: 'command', text: 'DO FORM x' }, hotkey: { key: 'N', ctrl: true } },
          { id: 'sep', prompt: '\\-', result: { type: 'bar' } },
          { id: 'exit', prompt: 'E\\<xit', result: { type: 'procedure', text: 'CLEAR EVENTS' } },
        ],
      },
      { id: 'help', prompt: '\\<Help', result: { type: 'submenu' }, children: [{ id: 'about', prompt: '\\<About', result: { type: 'command', text: '' } }] },
    ],
  };
}

export function sampleProject(): ProjectDocument {
  return {
    $schema: 'foxdev-project',
    version: 1,
    name: 'Sample',
    main: 'Form1.fxf',
    items: [
      { kind: 'form', path: 'Form1.fxf' },
      { kind: 'menu', path: 'Main.fxm' },
      { kind: 'program', path: 'main.prg' },
    ],
  };
}
