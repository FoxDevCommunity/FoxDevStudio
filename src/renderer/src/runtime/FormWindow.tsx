/**
 * A running form: VFP window chrome around live controls. Unlike the Milestone-1 preview this
 * is not a snapshot; the controls read the object model, so code that changes a property
 * updates the window immediately.
 */

import type { PointerEvent as ReactPointerEvent, ReactNode } from 'react';
import { Text } from '@fluentui/react-components';
import type { FormInstance } from '@shared/runtime/objectModel';
import { colorCss } from '../designer/surfaces';
import { RuntimeControl } from './RuntimeControl';
import { useRuntimeObject } from './useRuntimeObject';
import './runtime.css';

export function FormWindow({ form }: { form: FormInstance }) {
  const props = useRuntimeObject(form);
  if (props['Visible'] === false) return null;

  const width = Number(props['Width'] ?? 400);
  const height = Number(props['Height'] ?? 300);
  const caption = String(props['Caption'] ?? form.name);
  const autoCenter = props['AutoCenter'] === true;
  const left = Number(props['Left'] ?? 0);
  const top = Number(props['Top'] ?? 0);

  const close = () => void form.desktop.releaseForm(form, { queryUnload: true });

  return (
    <div
      role="dialog"
      aria-label={caption}
      data-runtime-form={form.name}
      style={{
        // AutoCenter forms flow in the document centred horizontally: centring them vertically
        // would push the title bar above the top edge whenever the pane is shorter than the form
        ...(autoCenter ? { position: 'relative', margin: '16px auto' } : { position: 'absolute', left, top }),
        width,
        border: '1px solid var(--colorNeutralStroke1)',
        borderRadius: 4,
        boxShadow: 'var(--shadow16)',
        background: 'var(--colorNeutralBackground1)',
        overflow: 'hidden',
      }}
    >
      <div
        className="fx-form-titlebar"
        onDoubleClick={() => void form.desktop.dispatch(form, 'DblClick')}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '4px 8px',
          background: 'var(--colorNeutralBackground3)',
          borderBottom: '1px solid var(--colorNeutralStroke2)',
          userSelect: 'none',
        }}
      >
        <Text size={200} weight="semibold" style={{ flex: 1 }}>
          {caption}
        </Text>
        {props['Closable'] !== false && (
          <button type="button" aria-label={`Close ${caption}`} onClick={close} className="fx-form-close">
            ✕
          </button>
        )}
      </div>
      <div
        data-testid="form-client"
        className="fx-form-client"
        onClick={() => void form.desktop.dispatch(form, 'Click')}
        style={{ position: 'relative', width, height, background: colorCss(props['BackColor']) }}
      >
        <FormDrawings form={form} />
        {form.children.map((child) => (
          <RuntimeControl key={child.handle} obj={child} />
        ))}
      </div>
      {sizable(props['BorderStyle']) && <ResizeGrip form={form} />}
    </div>
  );
}

/**
 * What Box, Line, Circle and PSet drew on the form. They go under the controls, as they do in
 * Visual FoxPro: a form draws on its own surface and the controls sit on top of it.
 */
function FormDrawings({ form }: { form: FormInstance }): ReactNode {
  if (form.drawings.length === 0) return null;
  return (
    <svg
      aria-hidden
      data-testid="form-drawings"
      style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}
      width="100%"
      height="100%"
    >
      {form.drawings.map((d, i) => {
        const stroke = colorCss(d.colour);
        const dash = d.style === 1 ? '6 3' : d.style === 2 ? '2 3' : d.style === 3 ? '6 3 2 3' : undefined;
        const common = { stroke, strokeWidth: d.width, strokeDasharray: dash, fill: 'none' as const };
        const key = `${i}-${d.shape}`;
        if (d.shape === 'box') {
          return (
            <rect
              key={key}
              x={Math.min(d.x1, d.x2)}
              y={Math.min(d.y1, d.y2)}
              width={Math.abs(d.x2 - d.x1)}
              height={Math.abs(d.y2 - d.y1)}
              {...common}
            />
          );
        }
        if (d.shape === 'circle') {
          return <ellipse key={key} cx={d.x1} cy={d.y1} rx={d.radius ?? 0} ry={(d.radius ?? 0) * (d.aspect || 1)} {...common} />;
        }
        if (d.shape === 'text') {
          // Print writes on the form where CurrentX and CurrentY say, in the form's own font
          return (
            <text key={key} x={d.x1} y={d.y1} fill={stroke} fontSize={Number(form.get('FontSize') ?? 9) * 1.33} dominantBaseline="hanging">
              {d.text ?? ''}
            </text>
          );
        }
        if (d.shape === 'point') {
          return <rect key={key} x={d.x1} y={d.y1} width={Math.max(1, d.width)} height={Math.max(1, d.width)} fill={stroke} />;
        }
        return <line key={key} x1={d.x1} y1={d.y1} x2={d.x2} y2={d.y2} {...common} />;
      })}
    </svg>
  );
}

/** VFP's BorderStyle: 3 (sizable) is the default; 0, 1 and 2 are fixed. */
function sizable(borderStyle: unknown): boolean {
  return borderStyle === undefined || borderStyle === null || borderStyle === 3;
}

/**
 * The corner a user drags to resize the form. The form's Width and Height are what change -
 * they are the object's properties, so anchored controls and the program's own Resize code
 * see the new size the way they would in VFP.
 */
function ResizeGrip({ form }: { form: FormInstance }) {
  const start = (e: ReactPointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    const origin = { x: e.clientX, y: e.clientY, w: Number(form.get('Width') ?? 400), h: Number(form.get('Height') ?? 300) };
    const grip = e.currentTarget;
    grip.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      form.set('Width', Math.max(120, Math.round(origin.w + ev.clientX - origin.x)));
      form.set('Height', Math.max(60, Math.round(origin.h + ev.clientY - origin.y)));
    };
    const stop = () => {
      grip.removeEventListener('pointermove', move);
      grip.removeEventListener('pointerup', stop);
      void form.desktop.dispatch(form, 'Resize');
    };
    grip.addEventListener('pointermove', move);
    grip.addEventListener('pointerup', stop);
  };
  return (
    <div
      aria-label="Resize"
      onPointerDown={start}
      style={{
        position: 'absolute',
        right: 0,
        bottom: 0,
        width: 16,
        height: 16,
        cursor: 'nwse-resize',
        background: 'linear-gradient(135deg, transparent 50%, var(--colorNeutralStroke1) 50%, var(--colorNeutralStroke1) 60%, transparent 60%, transparent 75%, var(--colorNeutralStroke1) 75%, var(--colorNeutralStroke1) 85%, transparent 85%)',
      }}
    />
  );
}
