import type { CSSProperties, FC, ReactNode } from 'react';
import type { ControlNode, ControlType, PropValue } from '@shared/form/schema';
import { toHex } from '@shared/form/color';
import { getProp } from '@shared/registry';
import { resolveIcon } from '../icons';
import { useFormDesigner } from '../store/FormDesignerContext';

export interface DesignSurfaceProps {
  node: ControlNode;
  /** Descriptor defaults merged with the node's props. */
  resolved: Record<string, PropValue>;
  /** Renders the ControlSurfaces of a container's children (relative to the element that calls it). */
  renderChildren: (parent: ControlNode) => ReactNode;
}

export const PAGEFRAME_TAB_HEIGHT = 24;

export function colorCss(v: PropValue | undefined): string | undefined {
  // A colour the object has not got is not black. A ListBox has no BackColor at all - the
  // product gives it ItemBackColor instead - and painting the absent one black turned every
  // list on every form into a black rectangle. Nothing is nothing: the element is left to
  // whatever it would have been.
  return typeof v === 'number' ? toHex(v) : undefined;
}

/**
 * What to ask the browser for when a form names a font.
 *
 * Visual FoxPro forms were laid out in fonts that Windows no longer ships: MS Sans Serif is the
 * default of everything written before about 2000, and a browser given that name falls back to
 * something wider, so every caption overflows the control it was measured for. Windows still has
 * Microsoft Sans Serif, which is the same metrics, so each old name is followed by its living
 * equivalent and then by a generic.
 */
const FONT_STACKS: Record<string, string> = {
  'ms sans serif': '"Microsoft Sans Serif", Tahoma, sans-serif',
  'ms serif': '"Times New Roman", serif',
  'ms dialog': '"Microsoft Sans Serif", Tahoma, sans-serif',
  helv: '"Microsoft Sans Serif", Tahoma, sans-serif',
  system: '"Microsoft Sans Serif", Tahoma, sans-serif',
  fixedsys: 'Consolas, "Courier New", monospace',
  terminal: 'Consolas, "Courier New", monospace',
  courier: '"Courier New", monospace',
};

/** The family to render a control in: what it asks for, then something that exists. */
export function fontFamilyCss(name: PropValue | undefined): string {
  const asked = String(name ?? 'Arial').trim();
  const known = FONT_STACKS[asked.toLowerCase()];
  if (known) return known;
  return `"${asked}", "Microsoft Sans Serif", Tahoma, sans-serif`;
}

export function fontStyle(r: Record<string, PropValue>): CSSProperties {
  return {
    fontFamily: fontFamilyCss(r['FontName']),
    fontSize: `${Number(r['FontSize'] ?? 9) * 1.333}px`,
    // Windows lays a line out at the font's own height, about 1.2 times its size; a browser
    // leaves it to the theme, and Fluent's is 20px whatever the font. Two lines of an 8pt label
    // then need 40 pixels where Visual FoxPro gave it 26, and the second one is cut in half.
    lineHeight: 1.2,
    fontWeight: r['FontBold'] ? 'bold' : 'normal',
    fontStyle: r['FontItalic'] ? 'italic' : 'normal',
    textDecoration: [r['FontUnderline'] ? 'underline' : '', r['FontStrikethru'] ? 'line-through' : ''].join(' ').trim() || 'none',
  };
}

function alignCss(v: PropValue | undefined): CSSProperties['textAlign'] {
  return v === 1 ? 'right' : v === 2 ? 'center' : 'left';
}

function textColors(r: Record<string, PropValue>, opaque = true): CSSProperties {
  const disabled = r['Enabled'] === false;
  return {
    color: colorCss(disabled ? r['DisabledForeColor'] : r['ForeColor']),
    background: opaque && r['BackStyle'] !== 0 ? colorCss(disabled ? r['DisabledBackColor'] : r['BackColor']) : 'transparent',
  };
}

const Label: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div
    className="fx-surface"
    style={{
      ...fontStyle(r),
      ...textColors(r, true),
      textAlign: alignCss(r['Alignment']),
      whiteSpace: r['WordWrap'] ? 'normal' : 'nowrap',
      border: r['BorderStyle'] === 1 ? '1px solid #000' : 'none',
      padding: '1px 2px',
    }}
  >
    {String(r['Caption'] ?? '')}
  </div>
);

const TextBox: FC<DesignSurfaceProps> = ({ resolved: r }) => {
  const value = String(r['Value'] ?? '');
  const pw = String(r['PasswordChar'] ?? '');
  return (
    <div
      className={`fx-surface ${r['BorderStyle'] === 0 ? '' : r['SpecialEffect'] === 1 ? 'fx-plain' : 'fx-3d'}`}
      style={{ ...fontStyle(r), ...textColors(r), textAlign: alignCss(r['Alignment'] === 3 ? 0 : r['Alignment']), padding: `1px ${Number(r['Margin'] ?? 2)}px`, border: r['BorderStyle'] === 0 ? 'none' : r['SpecialEffect'] === 1 ? '1px solid #7a7a7a' : undefined }}
    >
      {pw ? pw.repeat(value.length) : value}
    </div>
  );
};

const EditBox: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div className="fx-surface fx-3d" style={{ ...fontStyle(r), ...textColors(r), whiteSpace: 'pre-wrap', padding: 2, display: 'flex' }}>
    <div style={{ flex: 1, overflow: 'hidden' }}>{String(r['Value'] ?? '')}</div>
    {r['ScrollBars'] === 2 && <div style={{ width: 14, background: '#e6e3d8', borderLeft: '1px solid #c0c0c0' }} />}
  </div>
);

const CommandButton: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div
    className={`fx-surface ${r['SpecialEffect'] === 1 ? '' : 'fx-raised'}`}
    style={{
      ...fontStyle(r),
      color: colorCss(r['Enabled'] === false ? r['DisabledForeColor'] : r['ForeColor']),
      background: colorCss(r['BackColor']),
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      border: r['SpecialEffect'] === 1 ? '1px solid #000' : undefined,
      outline: r['Default'] ? '1px solid #000' : 'none',
      outlineOffset: -2,
    }}
  >
    {String(r['Caption'] ?? '')}
  </div>
);

const CheckBox: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div className="fx-surface" style={{ ...fontStyle(r), ...textColors(r), display: 'flex', alignItems: 'center', gap: 4, flexDirection: r['Alignment'] === 1 ? 'row-reverse' : 'row', justifyContent: r['Alignment'] === 1 ? 'flex-end' : 'flex-start' }}>
    <span style={{ width: 13, height: 13, border: '1px solid #1c5180', background: '#fff', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', fontSize: 11, flex: 'none' }}>
      {r['Value'] === true || r['Value'] === 1 ? '✓' : ''}
    </span>
    <span>{String(r['Caption'] ?? '')}</span>
  </div>
);

const OptionButton: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div className="fx-surface" style={{ ...fontStyle(r), ...textColors(r), display: 'flex', alignItems: 'center', gap: 4 }}>
    <span style={{ width: 12, height: 12, borderRadius: '50%', border: '1px solid #1c5180', background: '#fff', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', flex: 'none' }}>
      {r['Value'] === 1 ? <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#1c5180' }} /> : null}
    </span>
    <span>{String(r['Caption'] ?? '')}</span>
  </div>
);

const GroupBox: FC<DesignSurfaceProps> = ({ node, resolved: r, renderChildren }) => (
  <div className="fx-surface" style={{ ...textColors(r), border: r['BorderStyle'] === 0 ? 'none' : '1px solid #7a7a7a' }}>
    <div className="fx-container-client" data-container-id={node.id}>
      {renderChildren(node)}
    </div>
  </div>
);

const ComboBox: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div className="fx-surface fx-3d" style={{ ...fontStyle(r), ...textColors(r), display: 'flex' }}>
    <div style={{ flex: 1, padding: '1px 3px', overflow: 'hidden' }}>{String(r['DisplayValue'] || r['Value'] || '')}</div>
    <div style={{ width: 16, background: '#e6e3d8', borderLeft: '1px solid #7a7a7a', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 8 }}>▼</div>
  </div>
);

function valueListItems(r: Record<string, PropValue>): string[] {
  return r['RowSourceType'] === 1 && typeof r['RowSource'] === 'string' && r['RowSource'] ? r['RowSource'].split(',').map((s) => s.trim()) : [];
}

const ListBox: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div className="fx-surface fx-3d" style={{ ...fontStyle(r), ...textColors(r), padding: 2 }}>
    {valueListItems(r).map((item, i) => (
      <div key={i} style={{ padding: '0 2px' }}>
        {item}
      </div>
    ))}
  </div>
);

const Spinner: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div className="fx-surface fx-3d" style={{ ...fontStyle(r), ...textColors(r), display: 'flex' }}>
    <div style={{ flex: 1, padding: '1px 3px', textAlign: alignCss(r['Alignment'] === 3 ? 1 : r['Alignment']) }}>{String(r['Value'] ?? 0)}</div>
    <div style={{ width: 14, background: '#e6e3d8', borderLeft: '1px solid #7a7a7a', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'space-around', fontSize: 6 }}>
      <span>▲</span>
      <span>▼</span>
    </div>
  </div>
);

const Shape: FC<DesignSurfaceProps> = ({ resolved: r }) => (
  <div
    className="fx-surface"
    style={{
      background: r['BackStyle'] === 0 || r['FillStyle'] === 1 ? 'transparent' : colorCss(r['FillColor'] ?? r['BackColor']),
      border: r['BorderStyle'] === 0 ? 'none' : `${Math.max(1, Number(r['BorderWidth'] ?? 1))}px ${r['BorderStyle'] === 2 ? 'dashed' : r['BorderStyle'] === 3 ? 'dotted' : 'solid'} ${colorCss(r['BorderColor'])}`,
      borderRadius: `${Math.min(50, Number(r['Curvature'] ?? 0) / 2)}%`,
    }}
  />
);

const Line: FC<DesignSurfaceProps> = ({ node, resolved: r }) => {
  const w = Number(getProp(node, 'Width'));
  const h = Number(getProp(node, 'Height'));
  const color = colorCss(r['BorderColor']);
  const bw = Math.max(1, Number(r['BorderWidth'] ?? 1));
  if (w > 0 && h > 0) {
    const slash = r['LineSlant'] === '/';
    return (
      <svg className="fx-surface" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" style={{ display: 'block' }}>
        <line x1={0} y1={slash ? h : 0} x2={w} y2={slash ? 0 : h} stroke={color} strokeWidth={bw} vectorEffect="non-scaling-stroke" />
      </svg>
    );
  }
  return <div className="fx-surface" style={{ background: color, minWidth: bw, minHeight: bw }} />;
};

const ImageIcon = resolveIcon('Image');
const TimerIcon = resolveIcon('Timer');
const CubeIcon = resolveIcon('Cube');

const Image: FC<DesignSurfaceProps> = ({ resolved: r }) => {
  const pic = String(r['Picture'] ?? '');
  return (
    <div className="fx-surface" style={{ background: r['BackStyle'] === 0 ? 'transparent' : colorCss(r['BackColor']), border: r['BorderStyle'] === 1 ? '1px solid #000' : '1px dashed #9a9a9a', display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', color: '#666', fontSize: 11 }}>
      <ImageIcon />
      {pic && <span style={{ maxWidth: '100%', overflow: 'hidden', textOverflow: 'ellipsis' }}>{pic}</span>}
    </div>
  );
};

const Container: FC<DesignSurfaceProps> = ({ node, resolved: r, renderChildren }) => (
  <div
    className={`fx-surface ${r['BorderStyle'] === 0 ? '' : r['SpecialEffect'] === 0 ? 'fx-raised' : r['SpecialEffect'] === 1 ? 'fx-3d' : ''}`}
    style={{ background: r['BackStyle'] === 0 ? 'transparent' : colorCss(r['BackColor']), border: r['BorderStyle'] !== 0 && r['SpecialEffect'] === 2 ? `${Number(r['BorderWidth'] ?? 1)}px solid #7a7a7a` : undefined }}
  >
    <div className="fx-container-client" data-container-id={node.id}>
      {renderChildren(node)}
    </div>
  </div>
);

const PageFrame: FC<DesignSurfaceProps> = ({ node, resolved: r, renderChildren }) => {
  const active = useFormDesigner((s) => s.activePages[node.id] ?? 0);
  const setActivePage = useFormDesigner((s) => s.setActivePage);
  const select = useFormDesigner((s) => s.select);
  const pages = node.children ?? [];
  const page = pages[Math.min(active, Math.max(0, pages.length - 1))];
  return (
    <div className="fx-surface" style={{ border: r['BorderStyle'] === 0 ? 'none' : '1px solid #7a7a7a', ...fontStyle(r) }}>
      {r['Tabs'] !== false && (
        <div className="fx-tabs" role="tablist">
          {pages.map((p, i) => (
            <div
              key={p.id}
              role="tab"
              aria-selected={i === active}
              data-page-id={p.id}
              className={`fx-tab ${i === active ? 'fx-tab-active' : ''}`}
              onPointerDown={(e) => {
                e.stopPropagation();
                setActivePage(node.id, i);
                select([p.id]);
              }}
            >
              {String(getProp(p, 'Caption') ?? p.name)}
            </div>
          ))}
        </div>
      )}
      {page && (
        <div className="fx-page-client" data-container-id={page.id} data-page-id={page.id} style={{ background: colorCss(getProp(page, 'BackColor')) }}>
          {renderChildren(page)}
        </div>
      )}
    </div>
  );
};

const Grid: FC<DesignSurfaceProps> = ({ node, resolved: r }) => {
  const columns = node.children ?? [];
  const headerH = Number(r['HeaderHeight'] ?? 17);
  const rowH = Number(r['RowHeight'] ?? 17);
  const height = Number(getProp(node, 'Height'));
  const rows = Math.max(0, Math.floor((height - headerH - 2) / rowH));
  const marks = (r['DeleteMark'] ? 12 : 0) + (r['RecordMark'] ? 12 : 0);
  return (
    <div className="fx-surface fx-3d" style={{ ...fontStyle(r), background: colorCss(r['BackColor']) }}>
      <div className="fx-grid-header" style={{ height: headerH }}>
        {marks > 0 && <div className="fx-grid-cell" style={{ width: marks, flex: 'none' }} />}
        {columns.map((col) => {
          const header = col.children?.find((c) => c.type === 'Header');
          return (
            <div key={col.id} className="fx-grid-cell" data-column-id={col.id} style={{ width: Number(getProp(col, 'Width')), flex: 'none', textAlign: 'center' }}>
              {header ? String(getProp(header, 'Caption')) : col.name}
            </div>
          );
        })}
      </div>
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="fx-grid-row" style={{ height: rowH }}>
          {marks > 0 && <div className="fx-grid-cell" style={{ width: marks, flex: 'none', background: '#ece9d8' }} />}
          {columns.map((col) => (
            <div key={col.id} className="fx-grid-cell" style={{ width: Number(getProp(col, 'Width')), flex: 'none' }} />
          ))}
        </div>
      ))}
    </div>
  );
};

const Timer: FC<DesignSurfaceProps> = () => (
  <div className="fx-surface" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#ece9d8', border: '1px solid #7a7a7a', fontSize: 16 }}>
    <TimerIcon />
  </div>
);

/**
 * A Visual FoxPro object with no appearance: a custom, a session, a hyperlink. VFP draws these
 * as a small icon on the form, and so does this, so they can be selected and edited.
 */
const NonVisual: FC<DesignSurfaceProps> = ({ node }) => (
  <div
    className="fx-surface"
    title={node.name}
    style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#ece9d8', border: '1px solid #7a7a7a', fontSize: 16 }}
  >
    <CubeIcon />
  </div>
);

/**
 * A control this designer cannot draw but keeps whole: an ActiveX control, a toolbar separator.
 * The box says what it is, so the form reads correctly even though the control is inert.
 */
const Placeholder: FC<DesignSurfaceProps> = ({ node }) => (
  <div
    className="fx-surface"
    title={node.name}
    style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: 'repeating-linear-gradient(45deg, #f3f2f1, #f3f2f1 6px, #e8e6e4 6px, #e8e6e4 12px)',
      border: '1px dashed #9a9a9a',
      color: '#605e5c',
      fontSize: 11,
      overflow: 'hidden',
    }}
  >
    {node.type === 'Separator' ? '' : node.name}
  </div>
);

const Nothing: FC<DesignSurfaceProps> = () => null;

/** Design-time renderers, one per control type. Page/Column/Header are drawn by their parents. */
export const designSurfaces: Record<ControlType, FC<DesignSurfaceProps>> = {
  Label,
  TextBox,
  EditBox,
  CommandButton,
  CheckBox,
  OptionGroup: GroupBox,
  OptionButton,
  ComboBox,
  ListBox,
  Spinner,
  Shape,
  Line,
  Image,
  Container,
  PageFrame,
  Page: Nothing,
  Grid,
  Column: Nothing,
  Header: Nothing,
  Timer,
  CommandGroup: GroupBox,
  Custom: NonVisual,
  Session: NonVisual,
  Hyperlink: NonVisual,
  Collection: NonVisual,
  Toolbar: Container,
  Separator: Placeholder,
  OleControl: Placeholder,
  OleBoundControl: Placeholder,
};
