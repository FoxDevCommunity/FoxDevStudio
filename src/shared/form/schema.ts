import { z } from 'zod';

/** A VFP property value. Colors are RGB() ints, expressions are strings. */
export type PropValue = string | number | boolean | null;

export const CONTROL_TYPES = [
  'Label',
  'TextBox',
  'EditBox',
  'CommandButton',
  'CheckBox',
  'OptionGroup',
  'OptionButton',
  'ComboBox',
  'ListBox',
  'Spinner',
  'Shape',
  'Line',
  'Image',
  'Container',
  'PageFrame',
  'Page',
  'Grid',
  'Column',
  'Header',
  'Timer',
  'CommandGroup',
  // Visual FoxPro base classes a real form carries that are not ordinary controls
  'Custom',
  'Session',
  'Hyperlink',
  'Collection',
  'Toolbar',
  'Separator',
  'OleControl',
  'OleBoundControl',
] as const;
export type ControlType = (typeof CONTROL_TYPES)[number];

export interface ControlNode {
  /** Designer-stable id (nanoid). Never a VFP property; VFP identity is `name`. */
  id: string;
  type: ControlType;
  /** VFP Name (Command1, Text1...). Unique within the form, case-insensitive. */
  name: string;
  /** VFP property names, SPARSE: only values that differ from the descriptor defaults. */
  props: Record<string, PropValue>;
  /** Event/method name -> source text. Empty string means "no code". */
  methods: Record<string, string>;
  /** Present on containers: Container, PageFrame(Page), Page, Grid(Column), Column(Header, control), OptionGroup, CommandGroup. */
  children?: ControlNode[];
}

export interface FormNode {
  name: string;
  props: Record<string, PropValue>;
  methods: Record<string, string>;
  children: ControlNode[];
}

/**
 * One table a form opens for itself. Visual FoxPro calls this a cursor of the form's data
 * environment: it is opened before Load and closed after Unload, which is why so much form code
 * simply assumes a table is there.
 */
export interface FormCursor {
  /** The name the table answers to; `SELECT customer` and `customer.name` use this. */
  alias: string;
  /** The file, as the form names it - usually relative to the project. */
  source: string;
  /** `ORDER` to set after opening, when the form asked for one. */
  order?: string;
  exclusive?: boolean;
  /**
   * The database container the table belongs to, as the form names it. A table in a database is
   * named without a path (`CursorSource = "customer"`), and the container is the only thing that
   * says where it is.
   */
  database?: string;
}

/**
 * One relation a form's data environment sets up between two of its tables.
 *
 * A relation is what makes a grid of order lines follow the order the user is on: the child
 * table's record pointer moves whenever the parent's does, matched on an index. VFP writes one
 * `relation` row per link, and the form sets them all up before Load.
 */
export interface FormRelation {
  /** The alias whose record pointer leads. */
  parent: string;
  /** The alias that follows it. */
  child: string;
  /** The expression matched against the child's index, evaluated in the parent. */
  expression: string;
  /** The tag the child is ordered by while the relation holds. */
  childOrder?: string;
  /** One parent record to many child records: the child is stepped through, not just sought. */
  oneToMany?: boolean;
}

/**
 * A formset: the container Visual FoxPro puts round several forms so they are shown, activated
 * and released together, and which `THISFORMSET` reaches from anywhere inside any of them.
 *
 * It has no designer of its own here - a form is what is drawn - so each of its member forms is
 * a document, and every one of them carries the same record of the formset that holds them. That
 * is enough to build the whole thing again from any member: the formset's own name, the values
 * and code written on it, and which forms belong to it, in the order the file defines them.
 */
export interface VfpFormset {
  /** The formset's `Name`, which is what `THISFORMSET.Name` answers. */
  name: string;
  /** Its own property values, sparse, exactly as a form's are. */
  props: Record<string, PropValue>;
  /** Its methods and event handlers, by name; empty source means there is none. */
  methods: Record<string, string>;
  /** Every member form's name, in the order the file defines them. */
  forms: string[];
}

/**
 * Reserved for the VFP importer and user-defined members.
 *
 * Every document format of ours carries this, not just the form: a class library and a report say
 * where they were converted from in exactly the same words, so one piece of code can ask any
 * document whether an older importer made it.
 */
export interface DocumentMeta {
  vfp?: {
    classLib?: string;
    baseClass?: string;
    /**
     * The header file this form's own file named, as the file wrote it. Every `#DEFINE` in it is
     * in scope in every method the form holds, contained controls included, exactly as though
     * each method began with an `#INCLUDE` of it. Visual FoxPro keeps it in the eighth reserved
     * field of the `.scx`, and looks for the file beside the form before anywhere else.
     */
    include?: string;
    /**
     * The header file of every method that is compiled with a different one from `include`, by
     * the path `formMethodSources` gives the method (`_cryptapi.APISetup`, lower-cased; the
     * form's own methods under the bare event name). A method a control inherited from a class
     * library is compiled with that library's header and never with the form's - measured - so
     * the entry holds the header as a path from the library that brought the method in, and an
     * empty string says the method's own file named none at all.
     */
    includes?: Record<string, string>;
    reserved?: Record<string, string>;
    /**
     * Property values that are Visual FoxPro expressions rather than values written down, by
     * `objectPath.PropertyName`. `Picture = (HOME() + "graphics\\edit.bmp")` finds a bitmap
     * wherever VFP is installed and `Caption = (STR(RECNO()))` shows a record number; both are
     * worked out when the form loads, which is when VFP works them out.
     */
    expressions?: Record<string, string>;
    /**
     * Array properties an object adds for itself, by `objectPath.PropertyName`, as `[rows, cols]`
     * with `cols` 0 for a list of one dimension. VFP writes them among the custom members as
     * `^aIcon[5,2]`, and they have to exist before any method runs: `ALEN(thisform.oWindows)` on
     * a property that was never dimensioned is "Unknown member OWINDOWS", not an empty array.
     */
    arrays?: Record<string, [number, number]>;
    /**
     * The formset this form belongs to, when the file held one. Several forms shown together
     * are several documents here, and this is what says they are one thing.
     */
    formset?: VfpFormset;
    /**
     * The forms this document holds, when the document *is* a formset rather than a member of
     * one - a formset class in a `.vcx`. A formset's members are whole forms, which no control
     * tree can hold, so they are kept beside the tree rather than in it.
     */
    memberForms?: FormNode[];
    /**
     * The Visual FoxPro file this was converted from, relative to the project, and the version
     * of the importer that did it. Together they let the IDE notice a document made by an older
     * importer and convert it again, so a project imported before a fix picks the fix up.
     */
    source?: string;
    importer?: number;
  };
  userProps?: Record<string, PropValue>;
}

export interface FormDocument {
  $schema: 'foxdev-form';
  version: 1;
  form: FormNode;
  /** The tables the form opens when it loads, in the order it opens them. */
  data?: FormCursor[];
  /** The links between those tables, set up once they are all open. */
  relations?: FormRelation[];
  meta?: DocumentMeta;
}

export const FORM_SCHEMA_ID = 'foxdev-form' as const;
export const FORM_VERSION = 1 as const;

export const propValueSchema = z.union([z.string(), z.number(), z.boolean(), z.null()]);
export const propsSchema = z.record(z.string(), propValueSchema);
export const methodsSchema = z.record(z.string(), z.string());

export const controlNodeSchema: z.ZodType<ControlNode> = z.lazy(() =>
  z.object({
    id: z.string().min(1),
    type: z.enum(CONTROL_TYPES),
    name: z.string().min(1),
    props: propsSchema,
    methods: methodsSchema,
    children: z.array(controlNodeSchema).optional(),
  }),
);

export const formNodeSchema: z.ZodType<FormNode> = z.object({
  name: z.string().min(1),
  props: propsSchema,
  methods: methodsSchema,
  children: z.array(controlNodeSchema),
});

export const formRelationSchema: z.ZodType<FormRelation> = z.object({
  parent: z.string(),
  child: z.string(),
  expression: z.string(),
  childOrder: z.string().optional(),
  oneToMany: z.boolean().optional(),
});

export const formCursorSchema: z.ZodType<FormCursor> = z.object({
  alias: z.string(),
  source: z.string(),
  order: z.string().optional(),
  exclusive: z.boolean().optional(),
  database: z.string().optional(),
});

export const vfpFormsetSchema: z.ZodType<VfpFormset> = z.object({
  name: z.string(),
  props: propsSchema,
  methods: methodsSchema,
  forms: z.array(z.string()),
});

export const documentMetaSchema: z.ZodType<DocumentMeta> = z.object({
  vfp: z
    .object({
      classLib: z.string().optional(),
      baseClass: z.string().optional(),
      include: z.string().optional(),
      includes: z.record(z.string(), z.string()).optional(),
      reserved: z.record(z.string(), z.string()).optional(),
      expressions: z.record(z.string(), z.string()).optional(),
      arrays: z.record(z.string(), z.tuple([z.number(), z.number()])).optional(),
      formset: vfpFormsetSchema.optional(),
      memberForms: z.array(formNodeSchema).optional(),
      source: z.string().optional(),
      importer: z.number().optional(),
    })
    .optional(),
  userProps: propsSchema.optional(),
});

export const formDocumentSchema: z.ZodType<FormDocument> = z.object({
  $schema: z.literal(FORM_SCHEMA_ID),
  version: z.literal(FORM_VERSION),
  form: formNodeSchema,
  data: z.array(formCursorSchema).optional(),
  relations: z.array(formRelationSchema).optional(),
  meta: documentMetaSchema.optional(),
});

/** Result type used by all parsers (no exceptions for user data). */
export type ParseResult<T> = { ok: true; doc: T } | { ok: false; error: string };
