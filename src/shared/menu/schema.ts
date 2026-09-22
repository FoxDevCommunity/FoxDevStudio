import { z } from 'zod';

export const MENU_SCHEMA_ID = 'foxdev-menu' as const;
export const MENU_VERSION = 1 as const;

export const MENU_RESULT_TYPES = ['submenu', 'command', 'procedure', 'pad', 'bar'] as const;
export type MenuResultType = (typeof MENU_RESULT_TYPES)[number];

export const MENU_LOCATIONS = ['Replace', 'Append', 'Before', 'After'] as const;
export type MenuLocation = (typeof MENU_LOCATIONS)[number];

export interface MenuHotkey {
  key: string; // 'S', 'F5', 'Delete'
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  /** Text shown next to the prompt, e.g. "Ctrl+S". Defaults to formatHotkey(). */
  label?: string;
}

export interface MenuItem {
  id: string;
  /** VFP prompt: "\<File" marks the F hotkey, "\-" is a separator. */
  prompt: string;
  /** VFP pad/bar name (_mfile). Optional. */
  name?: string;
  result: { type: MenuResultType; text?: string };
  hotkey?: MenuHotkey;
  skipFor?: string;
  message?: string;
  enabled?: boolean;
  children?: MenuItem[];
}

export interface MenuDocument {
  $schema: 'foxdev-menu';
  version: 1;
  name: string;
  location: MenuLocation;
  /**
   * A shortcut menu: one that appears where the pointer is rather than along the top.
   *
   * Visual FoxPro generates `DEFINE POPUP <name> SHORTCUT RELATIVE FROM MROW(), MCOL()` for one
   * and activates it straight away, so running it shows it and waits for a choice. Its items are
   * the bars of that popup, not pads of a bar.
   */
  shortcut?: boolean;
  setup?: string;
  cleanup?: string;
  /** Top level items are menu pads, or the bars of the popup when this is a shortcut menu. */
  items: MenuItem[];
}

export const menuItemSchema: z.ZodType<MenuItem> = z.lazy(() =>
  z.object({
    id: z.string().min(1),
    prompt: z.string(),
    name: z.string().optional(),
    result: z.object({ type: z.enum(MENU_RESULT_TYPES), text: z.string().optional() }),
    hotkey: z
      .object({
        key: z.string().min(1),
        ctrl: z.boolean().optional(),
        alt: z.boolean().optional(),
        shift: z.boolean().optional(),
        label: z.string().optional(),
      })
      .optional(),
    skipFor: z.string().optional(),
    message: z.string().optional(),
    enabled: z.boolean().optional(),
    children: z.array(menuItemSchema).optional(),
  }),
);

export const menuDocumentSchema: z.ZodType<MenuDocument> = z.object({
  $schema: z.literal(MENU_SCHEMA_ID),
  version: z.literal(MENU_VERSION),
  name: z.string().min(1),
  location: z.enum(MENU_LOCATIONS),
  shortcut: z.boolean().optional(),
  setup: z.string().optional(),
  cleanup: z.string().optional(),
  items: z.array(menuItemSchema),
});

export function isSeparator(item: MenuItem): boolean {
  return item.prompt === '\\-';
}
