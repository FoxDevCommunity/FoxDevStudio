import { z } from 'zod';

export const PROJECT_SCHEMA_ID = 'foxdev-project' as const;
export const PROJECT_VERSION = 1 as const;

export const PROJECT_ITEM_KINDS = ['form', 'menu', 'program', 'class', 'database', 'table', 'report', 'other'] as const;
export type ProjectItemKind = (typeof PROJECT_ITEM_KINDS)[number];

export interface ProjectItem {
  kind: ProjectItemKind;
  /** Path relative to the project file, always with forward slashes. */
  path: string;
  excluded?: boolean;
}

export interface ProjectDocument {
  $schema: 'foxdev-project';
  version: 1;
  name: string;
  /** Relative path of the main program/form (VFP "Set Main"). */
  main?: string;
  items: ProjectItem[];
  settings?: { codePage?: number; debugInfo?: boolean };
}

export const projectDocumentSchema: z.ZodType<ProjectDocument> = z.object({
  $schema: z.literal(PROJECT_SCHEMA_ID),
  version: z.literal(PROJECT_VERSION),
  name: z.string().min(1),
  main: z.string().optional(),
  items: z.array(
    z.object({
      kind: z.enum(PROJECT_ITEM_KINDS),
      path: z.string().min(1),
      excluded: z.boolean().optional(),
    }),
  ),
  settings: z.object({ codePage: z.number().optional(), debugInfo: z.boolean().optional() }).optional(),
});

/**
 * File extension -> item kind.
 *
 * Both of a kind's spellings live here: the document of ours the designers write, and the Visual
 * FoxPro file that imports to it. A project that still refers to a `.frx` names a report as
 * surely as one that refers to an `.fxr`, and `REPORT FORM` reads either.
 */
export const EXTENSION_KINDS: Record<string, ProjectItemKind> = {
  '.fxf': 'form',
  '.fxm': 'menu',
  '.prg': 'program',
  '.fxc': 'class',
  '.dbc': 'database',
  '.dbf': 'table',
  '.fxr': 'report',
  '.fxl': 'report',
  '.frx': 'report',
  '.lbx': 'report',
};

/** The extension a designer of ours gives a new item of each kind. */
export const KIND_EXTENSIONS: Record<ProjectItemKind, string> = {
  form: '.fxf',
  menu: '.fxm',
  program: '.prg',
  class: '.fxc',
  database: '.dbc',
  table: '.dbf',
  report: '.fxr',
  other: '',
};

export function kindForPath(path: string): ProjectItemKind {
  const m = /\.[^./\\]+$/.exec(path);
  return (m && EXTENSION_KINDS[m[0].toLowerCase()]) || 'other';
}
