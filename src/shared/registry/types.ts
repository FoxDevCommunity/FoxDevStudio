import type { ControlType, PropValue } from '../form/schema';

export type PropEditor = 'text' | 'number' | 'boolean' | 'color' | 'enum' | 'font' | 'expression' | 'picture' | 'multiline';
export type PropCategory = 'Layout' | 'Appearance' | 'Data' | 'Behavior' | 'Other';
export type ToolboxGroup = 'Standard' | 'Container' | 'Data' | 'Other';

export interface EnumValue {
  value: number | string;
  label: string; // shown VFP style: "1 - Fixed Single"
}

export interface PropertyMeta {
  name: string;
  editor: PropEditor;
  category: PropCategory;
  default: PropValue;
  enumValues?: EnumValue[];
  readOnly?: boolean;
  /** The error the product raises when a program writes it, for one that is read-only. */
  refusesWrite?: number;
  /** The object answers to it, but Visual FoxPro's property sheet does not list it. */
  hidden?: boolean;
  min?: number;
  max?: number;
  description?: string;
}

export interface EventMeta {
  name: string;
  /** VFP parameter list text, e.g. "nKeyCode, nShiftAltCtrl". */
  params?: string;
  description?: string;
}

/** Anything with a property sheet and a methods list: the form and every control. */
export interface ObjectDescriptor {
  displayName: string;
  /** VFP BaseClass string used by the importer ("commandbutton"). */
  baseClass: string;
  properties: PropertyMeta[];
  events: EventMeta[];
  /** Methods the class answers to, as Visual FoxPro 9 lists them. */
  methods?: string[];
  /** Event opened by double-clicking the object in the designer. */
  defaultEvent: string;
}

export interface ControlDescriptor extends ObjectDescriptor {
  type: ControlType;
  toolboxGroup: ToolboxGroup;
  /** Fluent icon name resolved by the renderer (e.g. "TextT"). */
  icon: string;
  /** "Text" -> Text1, Text2 ... */
  namePrefix: string;
  defaultSize: { Width: number; Height: number };
  container?: {
    /** Child types allowed. 'visual' = any visual control. */
    accepts: ControlType[] | 'visual';
    /** Children created automatically on add; count may be tied to a property (PageCount). */
    autoChildren?: { type: ControlType; count: number; countProp?: string };
  };
  /** Timer: drawn as an icon in the designer, absent at runtime. */
  nonVisual?: boolean;
  /** Line: resize keeps it one-dimensional. */
  fixedAspect?: 'line';
  /** Page, Column, Header, OptionButton: only created through their parent. */
  hideInToolbox?: boolean;
}
