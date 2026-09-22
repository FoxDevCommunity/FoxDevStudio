/**
 * The dialogs a running program can put on screen. Each one parks the FoxPro fiber until the
 * user answers, which is what makes `IF MESSAGEBOX(...) = 6` read like blocking code even
 * though nothing blocks the UI thread.
 */

import { useEffect, useRef, useState } from 'react';
import { Button, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface, DialogTitle, Input, Text } from '@fluentui/react-components';
import type { VmValue } from '@shared/runtime/values';
import { promptToLabel } from '../../menu-designer/hotkey';
import { messageBoxButtons, useSessionStore, type PendingDialog, type PendingRead } from '../session';

/** MESSAGEBOX() and INPUTBOX(). Keyed by dialog id so each one starts with fresh state. */
export function RuntimeDialog() {
  const dialog = useSessionStore((s) => s.dialog);
  return dialog ? <DialogBox key={dialog.id} dialog={dialog} /> : null;
}

function DialogBox({ dialog }: { dialog: PendingDialog }) {
  const [text, setText] = useState(dialog.defaultText);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const answer = (value: string | number | null) => {
    useSessionStore.setState({ dialog: null });
    dialog.resolve(value);
  };

  const buttons = dialog.kind === 'message' ? messageBoxButtons(dialog.flags) : [];

  // GETKEY() waits for a keypress rather than a button
  if (dialog.kind === 'key') return <KeyDialog dialog={dialog} onAnswer={answer} />;
  if (dialog.kind === 'color') return <ColorDialog dialog={dialog} onAnswer={answer} />;
  if (dialog.kind === 'font') return <FontDialog dialog={dialog} onAnswer={answer} />;

  return (
    <Dialog open modalType="alert">
      <DialogSurface aria-label={dialog.title || 'FoxDev Studio'}>
        <DialogBody>
          <DialogTitle>{dialog.title || 'FoxDev Studio'}</DialogTitle>
          <DialogContent>
            <Text style={{ whiteSpace: 'pre-wrap' }}>{dialog.text}</Text>
            {dialog.kind === 'input' && (
              <Input
                ref={inputRef}
                aria-label="Input"
                value={text}
                onChange={(_e, d) => setText(d.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') answer(text);
                  if (e.key === 'Escape') answer('');
                }}
                style={{ marginTop: 12, width: '100%' }}
              />
            )}
          </DialogContent>
          <DialogActions>
            {dialog.kind === 'input' ? (
              <>
                <Button appearance="primary" onClick={() => answer(text)}>
                  OK
                </Button>
                <Button onClick={() => answer('')}>Cancel</Button>
              </>
            ) : (
              buttons.map((b, i) => (
                <Button key={b.value} appearance={i === 0 ? 'primary' : 'secondary'} onClick={() => answer(b.value)}>
                  {b.label}
                </Button>
              ))
            )}
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

/** GETKEY(): resolves with the code of the next key pressed. */
function KeyDialog({ dialog, onAnswer }: { dialog: PendingDialog; onAnswer(value: number): void }) {
  return (
    <Dialog open modalType="alert">
      <DialogSurface aria-label={dialog.title}>
        <DialogBody>
          <DialogTitle>{dialog.title}</DialogTitle>
          <DialogContent>
            <Input
              autoFocus
              aria-label="Press a key"
              value=""
              onChange={() => undefined}
              onKeyDown={(e) => {
                e.preventDefault();
                onAnswer(e.key.length === 1 ? e.key.charCodeAt(0) : e.keyCode || 0);
              }}
              placeholder={dialog.text}
              style={{ width: '100%' }}
            />
          </DialogContent>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

/** GETCOLOR(): resolves with a VFP RGB integer, or -1 when cancelled. */
function ColorDialog({ dialog, onAnswer }: { dialog: PendingDialog; onAnswer(value: number): void }) {
  const [hex, setHex] = useState(() => vfpColorToHex(Number(dialog.defaultText) || 0));
  return (
    <Dialog open modalType="alert">
      <DialogSurface aria-label={dialog.title}>
        <DialogBody>
          <DialogTitle>{dialog.title}</DialogTitle>
          <DialogContent>
            <input aria-label="Colour" type="color" value={hex} onChange={(e) => setHex(e.target.value)} style={{ width: 64, height: 32 }} />
            <Text style={{ marginLeft: 12 }}>{hex}</Text>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => onAnswer(hexToVfpColor(hex))}>
              OK
            </Button>
            <Button onClick={() => onAnswer(-1)}>Cancel</Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

const FONT_FAMILIES = ['Segoe UI', 'Arial', 'Consolas', 'Courier New', 'Times New Roman', 'Tahoma', 'Verdana'];

/** GETFONT(): resolves with "name,size,style", or "" when cancelled. */
function FontDialog({ dialog, onAnswer }: { dialog: PendingDialog; onAnswer(value: string): void }) {
  const [name, size, style] = dialog.defaultText.split(',');
  const [font, setFont] = useState(name || 'Segoe UI');
  const [points, setPoints] = useState(size || '9');
  const [weight, setWeight] = useState(style || '');
  return (
    <Dialog open modalType="alert">
      <DialogSurface aria-label={dialog.title}>
        <DialogBody>
          <DialogTitle>{dialog.title}</DialogTitle>
          <DialogContent>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <select aria-label="Font" value={font} onChange={(e) => setFont(e.target.value)}>
                {FONT_FAMILIES.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
              <Input aria-label="Size" value={points} onChange={(_e, d) => setPoints(d.value)} style={{ width: 72 }} />
              <select aria-label="Style" value={weight} onChange={(e) => setWeight(e.target.value)}>
                <option value="">Regular</option>
                <option value="B">Bold</option>
                <option value="I">Italic</option>
                <option value="BI">Bold Italic</option>
              </select>
            </div>
            <Text block style={{ marginTop: 12, fontFamily: font, fontSize: `${points}pt`, fontWeight: weight.includes('B') ? 'bold' : 'normal', fontStyle: weight.includes('I') ? 'italic' : 'normal' }}>
              The quick brown fox
            </Text>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => onAnswer([font, points, weight].join(','))}>
              OK
            </Button>
            <Button onClick={() => onAnswer('')}>Cancel</Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

/** VFP stores colours as r + g*256 + b*65536, the reverse of the usual hex order. */
function vfpColorToHex(value: number): string {
  const r = value & 0xff;
  const g = (value >> 8) & 0xff;
  const b = (value >> 16) & 0xff;
  return `#${[r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('')}`;
}

function hexToVfpColor(hex: string): number {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return r + g * 256 + b * 65536;
}

/** WAIT WINDOW: a corner toast, not a modal, exactly as VFP shows it. */
export function WaitWindowToast() {
  const wait = useSessionStore((s) => s.wait);
  if (!wait) return null;
  return (
    <div
      role="status"
      aria-label="Wait window"
      style={{
        position: 'absolute',
        top: 12,
        right: 12,
        zIndex: 40,
        padding: '8px 14px',
        background: 'var(--colorNeutralBackground1)',
        border: '1px solid var(--colorNeutralStroke1)',
        borderRadius: 4,
        boxShadow: 'var(--shadow8)',
        maxWidth: 320,
      }}
    >
      <Text size={200}>{wait.text}</Text>
    </div>
  );
}

/**
 * An unhandled runtime error. VFP offers Cancel/Ignore/Suspend; Ignore resumes at the next
 * statement, Cancel ends the program. "Edit" jumps to the offending line.
 */
export function ProgramErrorDialog({ onEdit }: { onEdit?(program: string, line: number): void }) {
  const report = useSessionStore((s) => s.errorReport);
  if (!report) return null;
  const { error, stack, resolve } = report;

  return (
    <Dialog open modalType="alert">
      <DialogSurface aria-label="Program Error">
        <DialogBody>
          <DialogTitle>Program Error</DialogTitle>
          <DialogContent>
            <Text block weight="semibold">
              {error.message}
            </Text>
            <Text block size={200} style={{ marginTop: 8 }}>
              Program: {error.program} Line: {error.line}
            </Text>
            {stack.length > 1 && (
              <Text block size={200} style={{ marginTop: 8, color: 'var(--colorNeutralForeground3)' }}>
                Called from {stack.slice(1).map((f) => `${f.program} (${f.line})`).join(' <- ')}
              </Text>
            )}
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => resolve('cancel')}>
              Cancel
            </Button>
            <Button onClick={() => resolve('ignore')}>Ignore</Button>
            {onEdit && (
              <Button
                onClick={() => {
                  resolve('cancel');
                  onEdit(error.program, error.line);
                }}
              >
                Edit
              </Button>
            )}
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

/**
 * The dialog a `READ` puts up, and the one a `MENU TO` puts up.
 *
 * Visual FoxPro's `READ` lets the user walk the fields `@ ... GET` drew on the character screen
 * and type into them. There is no character keyboard here, so the same fields are shown as a
 * form: what the user leaves in them goes back where it came from, in order.
 */
export function ReadDialog() {
  const read = useSessionStore((s) => s.read);
  return read ? <ReadBox key={read.id} read={read} /> : null;
}

function ReadBox({ read }: { read: PendingRead }) {
  const [values, setValues] = useState<string[]>(() => read.fields.map((f) => displayOf(f.value)));
  const first = useRef<HTMLInputElement>(null);

  useEffect(() => {
    first.current?.focus();
  }, []);

  const finish = (answer: VmValue) => {
    useSessionStore.setState({ read: null });
    read.resolve(answer);
  };

  // MENU TO: one choice out of the prompts `@ ... PROMPT` put up, or none
  if (read.fields.length === 0) {
    return (
      <Dialog open modalType="alert">
        <DialogSurface aria-label="Choose">
          <DialogBody>
            <DialogTitle>Choose</DialogTitle>
            <DialogContent>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                {read.prompts.map((prompt, i) => (
                  <Button key={prompt} appearance={i === 0 ? 'primary' : 'secondary'} onClick={() => finish(i + 1)}>
                    {promptToLabel(prompt).label}
                  </Button>
                ))}
              </div>
            </DialogContent>
            <DialogActions>
              <Button onClick={() => finish(0)}>Cancel</Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>
    );
  }

  return (
    <Dialog open modalType="alert">
      <DialogSurface aria-label="Read">
        <DialogBody>
          <DialogTitle>Enter</DialogTitle>
          <DialogContent>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {read.fields.map((field, i) => (
                <div key={field.name} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Text style={{ minWidth: 120 }}>{field.name}</Text>
                  <Input
                    ref={i === 0 ? first : undefined}
                    aria-label={field.name}
                    disabled={!field.enabled}
                    value={values[i] ?? ''}
                    onChange={(_e, d) => setValues((v) => v.map((old, at) => (at === i ? d.value : old)))}
                    style={{ flex: 1 }}
                  />
                </div>
              ))}
            </div>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={() => finish({ $arr: read.fields.map((f, i) => typedAs(f.value, values[i] ?? '')), $cols: 0 })}>
              OK
            </Button>
            <Button onClick={() => finish({ $arr: read.fields.map((f) => f.value), $cols: 0 })}>Cancel</Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}

/** A field's value as text to edit. */
function displayOf(value: VmValue): string {
  if (value === null || value === undefined) return '';
  if (typeof value === 'boolean') return value ? '.T.' : '.F.';
  return String(value);
}

/** What was typed, back as the kind of value the field held. */
function typedAs(was: VmValue, text: string): VmValue {
  if (typeof was === 'number') {
    const n = Number(text.trim());
    return Number.isFinite(n) ? n : 0;
  }
  if (typeof was === 'boolean') return /^(\.t\.|t|y|true|1)$/i.test(text.trim());
  return text;
}
