/**
 * The Windows Common Controls, provided by FoxDev rather than by COM.
 *
 * A Visual FoxPro form that shows a tree holds an ActiveX TreeView from `mscomctl.ocx`. That is
 * an OCX: it draws into a window of its own, which is something no page in a browser engine can
 * host, so the control cannot be shown even where COM is available. The controls that appear in
 * form after form are provided here instead - the same object model, the same members, backed by
 * FoxDev objects that React can draw.
 *
 * Everything here is an ordinary object addressable from FoxPro code: `oTree.Nodes.Add(...)`
 * reaches `TreeNodes.call('Add', ...)`, and the node it answers with is registered by the desktop
 * so `oNode.Image = "leaf"` reaches it in turn. None of it touches the DOM.
 */

import type { MemberEntry } from './objectModel';
import type { VmValue } from './values';

/** An object a FoxPro program can hold that is not a control of the form. */
export interface HostObject {
  /** What `Class` and `BaseClass` report. */
  readonly className: string;
  /**
   * True when a member may only be found by fetching it, so classifying a name must not.
   * A COM object is the case: asking it what a name is means invoking the name.
   */
  readonly lazy?: boolean;
  /** How a name answers, for a member the object does not hold an object under. */
  member(name: string): 'prop' | 'method' | 'none';
  get(name: string): VmValue | HostObject | undefined;
  set(name: string, value: VmValue): void;
  call(name: string, args: VmValue[]): VmValue | HostObject | Promise<VmValue> | undefined;
  /**
   * Everything the object has, for `AMEMBERS()` and `GETPEM()`. Optional: an object whose
   * members are worked out one name at a time - anything reached over COM - has no list to
   * hand over, and both functions then refuse the call rather than half-answer.
   */
  list?(): MemberEntry[];
}

/**
 * Something a program reads by number: `oProject.Files(1)`, `oAdapter.Tables(1)`,
 * `_VFP.Forms(1)`. Visual FoxPro writes them all the same way, so they all answer the same
 * way - `Count` says how many there are and calling it with a number gives one of them.
 */
export interface HostCollection extends HostObject {
  /** The nth of them, counting from one, or nothing when there is no such one. */
  at(index: number): HostObject | undefined;
}

/** Whether a value is one of those. */
export function isCollection(value: unknown): value is HostCollection {
  return typeof value === 'object' && value !== null && typeof (value as HostCollection).at === 'function';
}

/** What an emulated control needs from the runtime around it. */
export interface OleHost {
  /** The host object a `{ $obj: n }` reference points at, if it is one. */
  hosted(value: VmValue): HostObject | undefined;
  /** Something the control shows has changed; the form should be redrawn. */
  changed(): void;
}

/** True when an argument was left out: VFP passes `.F.` for a skipped one. */
function omitted(value: VmValue | undefined): boolean {
  return value === undefined || value === false || value === null || value === '';
}

function asText(value: VmValue | undefined): string {
  if (value === undefined || value === null || value === false) return '';
  return typeof value === 'string' ? value : String(value);
}

const lower = (name: string) => name.toLowerCase();

/** Answers a fixed set of members from a record of values. */
abstract class Members implements HostObject {
  abstract readonly className: string;
  protected abstract readonly props: Set<string>;
  protected abstract readonly methods: Set<string>;

  member(name: string): 'prop' | 'method' | 'none' {
    if (this.props.has(lower(name))) return 'prop';
    if (this.methods.has(lower(name))) return 'method';
    return 'none';
  }

  abstract get(name: string): VmValue | HostObject | undefined;
  abstract set(name: string, value: VmValue): void;
  abstract call(name: string, args: VmValue[]): VmValue | HostObject | undefined;
}

// ---------------------------------------------------------------- TreeView

const NODE_PROPS = new Set([
  'key',
  'text',
  'image',
  'selectedimage',
  'index',
  'expanded',
  'selected',
  'bold',
  'tag',
  'children',
  'parent',
  'child',
  'next',
  'previous',
  'root',
  'sorted',
  'forecolor',
  'backcolor',
  'fullpath',
  'checked',
  'visible',
  'expandedimage',
]);

export class TreeNode extends Members {
  readonly className = 'Node';
  protected readonly props = NODE_PROPS;
  protected readonly methods = new Set(['ensurevisible', 'createdragimage']);

  key = '';
  text = '';
  image: VmValue = '';
  selectedImage: VmValue = '';
  expanded = false;
  bold = false;
  checked = false;
  tag: VmValue = '';
  parent: TreeNode | null = null;
  readonly children: TreeNode[] = [];

  constructor(private readonly tree: TreeView) {
    super();
  }

  get index(): number {
    return this.tree.nodeList.indexOf(this) + 1;
  }

  get(name: string): VmValue | HostObject | undefined {
    switch (lower(name)) {
      case 'key':
        return this.key;
      case 'text':
        return this.text;
      case 'image':
        return this.image;
      case 'selectedimage':
      case 'expandedimage':
        return this.selectedImage;
      case 'index':
        return this.index;
      case 'expanded':
        return this.expanded;
      case 'selected':
        return this.tree.selected === this;
      case 'bold':
        return this.bold;
      case 'checked':
        return this.checked;
      case 'visible':
        return true;
      case 'tag':
        return this.tag;
      case 'children':
        return this.children.length;
      case 'parent':
        return this.parent ?? null;
      case 'child':
        return this.children[0] ?? null;
      case 'root':
        return this.tree.roots[0] ?? null;
      case 'next': {
        const siblings = this.parent ? this.parent.children : this.tree.roots;
        return siblings[siblings.indexOf(this) + 1] ?? null;
      }
      case 'previous': {
        const siblings = this.parent ? this.parent.children : this.tree.roots;
        const at = siblings.indexOf(this);
        return at > 0 ? siblings[at - 1]! : null;
      }
      case 'fullpath':
        return fullPath(this).join(this.tree.pathSeparator);
      case 'sorted':
        return false;
      default:
        return undefined;
    }
  }

  set(name: string, value: VmValue): void {
    switch (lower(name)) {
      case 'key':
        this.key = asText(value);
        break;
      case 'text':
        this.text = asText(value);
        break;
      case 'image':
        this.image = value;
        break;
      case 'selectedimage':
      case 'expandedimage':
        this.selectedImage = value;
        break;
      case 'expanded':
        this.expanded = value === true;
        break;
      case 'selected':
        if (value === true) this.tree.selected = this;
        break;
      case 'bold':
        this.bold = value === true;
        break;
      case 'checked':
        this.checked = value === true;
        break;
      case 'tag':
        this.tag = value;
        break;
      default:
        return;
    }
    this.tree.host.changed();
  }

  call(name: string, _args: VmValue[]): VmValue | HostObject | undefined {
    if (lower(name) === 'ensurevisible') {
      for (let node = this.parent; node; node = node.parent) node.expanded = true;
      this.tree.host.changed();
      return true;
    }
    return undefined;
  }
}

/** `oTree.Nodes`: the collection, which is also how nodes are made. */
export class TreeNodes extends Members {
  readonly className = 'Nodes';
  protected readonly props = new Set(['count']);
  protected readonly methods = new Set(['add', 'item', 'remove', 'clear']);

  constructor(private readonly tree: TreeView) {
    super();
  }

  get(name: string): VmValue | HostObject | undefined {
    return lower(name) === 'count' ? this.tree.nodeList.length : undefined;
  }

  set(): void {
    // a collection has nothing to write
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | undefined {
    switch (lower(name)) {
      case 'add':
        return this.add(args);
      case 'item':
        return this.item(args[0]);
      case 'remove': {
        const node = this.item(args[0]);
        if (node instanceof TreeNode) this.tree.remove(node);
        return null;
      }
      case 'clear':
        this.tree.clear();
        return null;
      default:
        // `oTree.Nodes(3)` and `oTree.Nodes("key")`: the collection's default member
        return args.length === 1 ? this.item(args[0]) : undefined;
    }
  }

  /**
   * `Add([relative], [relationship], [key], [text], [image], [selectedimage])`.
   *
   * `relationship` is VFP's `tvwFirst`/`tvwLast`/`tvwNext`/`tvwPrevious`/`tvwChild` (0-4); 4
   * makes the new node a child of `relative`, and anything else places it beside it. With no
   * relative the node goes to the top level, which is what `Add(,1,...)` means.
   */
  private add(args: VmValue[]): HostObject {
    const [relative, relationship, key, text, image, selectedImage] = args;
    const node = new TreeNode(this.tree);
    node.key = asText(key);
    node.text = asText(text);
    if (!omitted(image)) node.image = image!;
    if (!omitted(selectedImage)) node.selectedImage = selectedImage!;

    const anchor = omitted(relative) ? null : this.node(relative!);
    const how = typeof relationship === 'number' ? relationship : 1;
    if (anchor && how === 4) {
      node.parent = anchor;
      anchor.children.push(node);
    } else if (anchor) {
      node.parent = anchor.parent;
      const siblings = anchor.parent ? anchor.parent.children : this.tree.roots;
      const at = siblings.indexOf(anchor);
      siblings.splice(how === 0 ? 0 : how === 2 ? at + 1 : how === 3 ? at : siblings.length, 0, node);
    } else if (how === 0) {
      this.tree.roots.unshift(node);
    } else {
      this.tree.roots.push(node);
    }
    this.tree.nodeList.push(node);
    this.tree.host.changed();
    return node;
  }

  /** A node by 1-based index, by key, or by reference. */
  private item(which: VmValue | undefined): HostObject | null {
    if (which === undefined) return null;
    const node = this.node(which);
    return node ?? null;
  }

  private node(which: VmValue): TreeNode | null {
    if (typeof which === 'number') return this.tree.nodeList[which - 1] ?? null;
    if (typeof which === 'string') return this.tree.nodeList.find((n) => n.key.toLowerCase() === which.toLowerCase()) ?? null;
    const hosted = this.tree.host.hosted(which);
    return hosted instanceof TreeNode ? hosted : null;
  }
}

/** The TreeView itself: `MSComctlLib.TreeCtrl.2`. */
export class TreeView extends Members {
  readonly className = 'TreeView';
  protected readonly props = new Set([
    'nodes',
    'imagelist',
    'sorted',
    'selecteditem',
    'style',
    'linestyle',
    'indentation',
    'labeledit',
    'hideselection',
    'appearance',
    'borderstyle',
    'fullrowselect',
    'hottracking',
    'checkboxes',
    'pathseparator',
    'singlesel',
    'scroll',
    'olednddropmode',
  ]);
  protected readonly methods = new Set(['getvisiblecount', 'hittest', 'startlabeledit', 'refresh']);

  readonly nodes = new TreeNodes(this);
  /** Top-level nodes, in display order. */
  readonly roots: TreeNode[] = [];
  /** Every node in creation order, which is how `Nodes(n)` indexes them. */
  readonly nodeList: TreeNode[] = [];
  selected: TreeNode | null = null;
  imageList: HostObject | null = null;
  sorted = false;
  pathSeparator = '\\';
  style = 7;
  lineStyle = 0;
  checkboxes = false;

  constructor(readonly host: OleHost) {
    super();
  }

  get(name: string): VmValue | HostObject | undefined {
    switch (lower(name)) {
      case 'nodes':
        return this.nodes;
      case 'imagelist':
        return this.imageList;
      case 'sorted':
        return this.sorted;
      case 'selecteditem':
        return this.selected;
      case 'pathseparator':
        return this.pathSeparator;
      case 'style':
        return this.style;
      case 'linestyle':
        return this.lineStyle;
      case 'checkboxes':
        return this.checkboxes;
      case 'indentation':
        return 16;
      case 'labeledit':
      case 'appearance':
      case 'borderstyle':
      case 'hideselection':
      case 'fullrowselect':
      case 'hottracking':
      case 'singlesel':
      case 'scroll':
      case 'olednddropmode':
        return this.other.get(lower(name)) ?? false;
      default:
        return undefined;
    }
  }

  /** Members kept only so a program can write and read them back. */
  private readonly other = new Map<string, VmValue>();

  set(name: string, value: VmValue): void {
    switch (lower(name)) {
      case 'imagelist':
        this.imageList = this.host.hosted(value) ?? null;
        break;
      case 'sorted':
        this.sorted = value === true;
        this.sortAll();
        break;
      case 'selecteditem':
        this.selected = (this.host.hosted(value) as TreeNode | undefined) ?? null;
        break;
      case 'pathseparator':
        this.pathSeparator = asText(value) || '\\';
        break;
      case 'style':
        this.style = typeof value === 'number' ? value : this.style;
        break;
      case 'linestyle':
        this.lineStyle = typeof value === 'number' ? value : this.lineStyle;
        break;
      case 'checkboxes':
        this.checkboxes = value === true;
        break;
      default:
        this.other.set(lower(name), value);
        break;
    }
    this.host.changed();
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | undefined {
    switch (lower(name)) {
      case 'nodes':
        return this.nodes.call('item', args);
      case 'getvisiblecount':
        return this.visible().length;
      case 'hittest':
        return null;
      case 'refresh':
        this.host.changed();
        return null;
      default:
        return undefined;
    }
  }

  /** Nodes as they are drawn: depth first, skipping anything inside a collapsed node. */
  visible(): { node: TreeNode; depth: number }[] {
    const out: { node: TreeNode; depth: number }[] = [];
    const walk = (list: TreeNode[], depth: number) => {
      for (const node of list) {
        out.push({ node, depth });
        if (node.expanded) walk(node.children, depth + 1);
      }
    };
    walk(this.roots, 0);
    return out;
  }

  /** Selection as a click makes it. */
  select(node: TreeNode): void {
    this.selected = node;
  }

  /** Opens or closes a node, answering with what it became. */
  toggle(node: TreeNode): boolean {
    node.expanded = !node.expanded;
    return node.expanded;
  }

  remove(node: TreeNode): void {
    const siblings = node.parent ? node.parent.children : this.roots;
    const at = siblings.indexOf(node);
    if (at >= 0) siblings.splice(at, 1);
    const gone = [node, ...descendants(node)];
    for (const n of gone) {
      const i = this.nodeList.indexOf(n);
      if (i >= 0) this.nodeList.splice(i, 1);
      if (this.selected === n) this.selected = null;
    }
    this.host.changed();
  }

  clear(): void {
    this.roots.length = 0;
    this.nodeList.length = 0;
    this.selected = null;
    this.host.changed();
  }

  private sortAll(): void {
    if (!this.sorted) return;
    const byText = (a: TreeNode, b: TreeNode) => a.text.localeCompare(b.text);
    const walk = (list: TreeNode[]) => {
      list.sort(byText);
      for (const node of list) walk(node.children);
    };
    walk(this.roots);
  }
}

/** A node's text and that of every node above it, outermost first. */
function fullPath(node: TreeNode): string[] {
  return node.parent ? [...fullPath(node.parent), node.text] : [node.text];
}

function descendants(node: TreeNode): TreeNode[] {
  return node.children.flatMap((c) => [c, ...descendants(c)]);
}

// ---------------------------------------------------------------- ImageList

/** One image of an ImageList. Its picture is whatever the program loaded, if anything. */
export class ListImage extends Members {
  readonly className = 'ListImage';
  protected readonly props = new Set(['key', 'index', 'picture', 'tag']);
  protected readonly methods = new Set(['draw', 'extractIcon'.toLowerCase()]);

  constructor(
    public key: string,
    public index: number,
    public picture: VmValue,
  ) {
    super();
  }

  get(name: string): VmValue | undefined {
    switch (lower(name)) {
      case 'key':
        return this.key;
      case 'index':
        return this.index;
      case 'picture':
        return this.picture;
      default:
        return undefined;
    }
  }

  set(name: string, value: VmValue): void {
    if (lower(name) === 'key') this.key = asText(value);
    if (lower(name) === 'picture') this.picture = value;
  }

  call(): VmValue | undefined {
    return null;
  }
}

export class ListImages extends Members {
  readonly className = 'ListImages';
  protected readonly props = new Set(['count']);
  protected readonly methods = new Set(['add', 'item', 'remove', 'clear']);

  readonly items: ListImage[] = [];

  constructor(private readonly owner: ImageList) {
    super();
  }

  get(name: string): VmValue | undefined {
    return lower(name) === 'count' ? this.items.length : undefined;
  }

  set(): void {
    // nothing to write on a collection
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | undefined {
    switch (lower(name)) {
      case 'add': {
        // Add([index], [key], [picture])
        const [index, key, picture] = args;
        const at = typeof index === 'number' ? index : this.items.length + 1;
        const image = new ListImage(asText(key), at, picture ?? '');
        this.items.splice(Math.min(Math.max(at, 1), this.items.length + 1) - 1, 0, image);
        this.items.forEach((img, i) => (img.index = i + 1));
        this.owner.host.changed();
        return image;
      }
      case 'clear':
        this.items.length = 0;
        this.owner.host.changed();
        return null;
      case 'remove': {
        const found = this.find(args[0]);
        if (found) this.items.splice(this.items.indexOf(found), 1);
        this.items.forEach((img, i) => (img.index = i + 1));
        this.owner.host.changed();
        return null;
      }
      case 'item':
        return this.find(args[0]) ?? null;
      default:
        return args.length === 1 ? (this.find(args[0]) ?? null) : undefined;
    }
  }

  find(which: VmValue | undefined): ListImage | null {
    if (typeof which === 'number') return this.items[which - 1] ?? null;
    if (typeof which === 'string') return this.items.find((i) => i.key.toLowerCase() === which.toLowerCase()) ?? null;
    return null;
  }
}

/** `MSComctlLib.ImageListCtrl.2`: a list of pictures other controls draw by key or index. */
export class ImageList extends Members {
  readonly className = 'ImageList';
  protected readonly props = new Set(['listimages', 'imageheight', 'imagewidth', 'maskcolor', 'usemaskcolor', 'backcolor']);
  protected readonly methods = new Set(['overlay', 'listimages']);

  readonly listImages = new ListImages(this);
  imageHeight = 16;
  imageWidth = 16;

  constructor(readonly host: OleHost) {
    super();
  }

  get(name: string): VmValue | HostObject | undefined {
    switch (lower(name)) {
      case 'listimages':
        return this.listImages;
      case 'imageheight':
        return this.imageHeight;
      case 'imagewidth':
        return this.imageWidth;
      case 'maskcolor':
      case 'backcolor':
        return 0;
      case 'usemaskcolor':
        return true;
      default:
        return undefined;
    }
  }

  set(name: string, value: VmValue): void {
    if (lower(name) === 'imageheight' && typeof value === 'number') this.imageHeight = value;
    if (lower(name) === 'imagewidth' && typeof value === 'number') this.imageWidth = value;
    this.host.changed();
  }

  call(name: string, args: VmValue[]): VmValue | HostObject | undefined {
    if (lower(name) === 'listimages') return this.listImages.call('item', args);
    return undefined;
  }

  /** The picture a key or index stands for, for whoever is drawing. */
  picture(which: VmValue): VmValue {
    return this.listImages.find(which)?.picture ?? '';
  }
}

/** Builds the control an `OleClass` names, when this runtime provides one. */
export function createOleControl(kind: 'TreeView' | 'ImageList', host: OleHost): HostObject {
  return kind === 'TreeView' ? new TreeView(host) : new ImageList(host);
}
