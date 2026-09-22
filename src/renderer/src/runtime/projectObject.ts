/**
 * The Project object: the project that is open in the development environment, as a program
 * reaches it through `_VFP.ActiveProject`.
 *
 * Visual FoxPro lets a builder or a wizard read a project and act on it: add a file, set the
 * main one, build the application. The same project is open in this IDE's own explorer, so this
 * is that project seen from a program: the properties are read from the store, and the methods
 * do what the menu commands do.
 *
 * Source control is the sixth thing it does. Visual FoxPro talks to a registered SCC provider;
 * what is registered here is git, and `sourceControlProvider.ts` is where the project is put
 * under it. A project outside a working tree has no provider, and the six methods answer .F.
 * the way VFP's do when a project is not under source control.
 */

import { basename, dirname, join, relative } from '@shared/paths';
import type { HostObject } from '@shared/runtime/oleObjects';
import { Collection } from '@shared/runtime/collection';
import type { VmValue } from '@shared/runtime/values';
import { HostError } from '@shared/runtime/host';
import { kindForPath, type ProjectItem } from '@shared/project/schema';
import { useProjectStore } from '../stores/projectStore';
import { sourceControl } from './sourceControlProvider';

/** The six methods that talk to source control. */
const SOURCE_CONTROL_NAMES = ['ADDTOSCC', 'REMOVEFROMSCC', 'CHECKIN', 'CHECKOUT', 'UNDOCHECKOUT', 'GETLATESTVERSION'];
import { buildApp, buildExecutable } from './buildActions';
import { runtimeUi } from './session';

/** Error 1965: the operation is not available. */
const NO_PROVIDER = 1965;

/** What a File answers to. */
const FILE_METHODS = new Set(['MODIFY', 'RUN', 'REMOVE', 'SETMAIN', ...SOURCE_CONTROL_NAMES]);

/** What a Project answers to. */
const PROJECT_METHODS = new Set([
  'BUILD',
  'MODIFY',
  'RUN',
  'CLOSE',
  'SAVEAS',
  'SETMAIN',
  'CLEANUP',
  ...SOURCE_CONTROL_NAMES,
]);

/** The methods that go to the source-control provider rather than to the project. */
const SOURCE_CONTROL = new Set(SOURCE_CONTROL_NAMES);

/**
 * One of the six, on one file. VFP answers .T. when the provider did it and .F. when it could
 * not - including when the project is not under source control at all, which is what the
 * provider that stands in for one answers to everything.
 */
function callSourceControl(method: string, path: string, args: VmValue[]): Promise<VmValue> {
  const scc = sourceControl();
  switch (method) {
    case 'ADDTOSCC':
      return scc.addToScc(path);
    case 'REMOVEFROMSCC':
      return scc.removeFromScc(path);
    case 'CHECKOUT':
      return scc.checkOut(path);
    case 'CHECKIN':
      return scc.checkIn(path, typeof args[0] === 'string' ? args[0] : undefined);
    case 'UNDOCHECKOUT':
      return scc.undoCheckOut(path);
    default:
      return scc.getLatestVersion(path);
  }
}

/** What the `Type` of a file is, in the letter Visual FoxPro answers with. */
const TYPE_LETTERS: Record<ProjectItem['kind'], string> = {
  program: 'P',
  form: 'K',
  menu: 'M',
  report: 'R',
  class: 'V',
  table: 'D',
  database: 'd',
  other: 'Z',
};

/** One file in the project, as `oProject.Files(n)` hands it over. */
export class ProjectFile implements HostObject {
  readonly className = 'File';

  constructor(private readonly item: ProjectItem) {}

  member(name: string): 'prop' | 'method' | 'none' {
    if (this.get(name) !== undefined) return 'prop';
    return FILE_METHODS.has(name.toUpperCase()) ? 'method' : 'none';
  }

  /** Where the file is, as the project names it. */
  private get path(): string {
    return useProjectStore.getState().resolvePath(this.item.path);
  }

  get(name: string): VmValue | HostObject | undefined {
    switch (name.toUpperCase()) {
      case 'NAME':
        return this.path;
      case 'CLASS':
      case 'BASECLASS':
        return 'File';
      case 'TYPE':
        return TYPE_LETTERS[this.item.kind] ?? 'Z';
      case 'DESCRIPTION':
        return '';
      case 'EXCLUDE':
        return this.item.excluded === true;
      case 'LASTMODIFIED':
        return null;
      case 'SCCSTATUS':
        return sourceControl().statusOf(this.path);
      case 'READONLY':
        return false;
      default:
        return undefined;
    }
  }

  set(name: string, value: VmValue): void {
    if (name.toUpperCase() === 'EXCLUDE') {
      useProjectStore.getState().setExcluded(this.item.path, value === true);
    }
  }

  call(name: string, args: VmValue[]): VmValue | Promise<VmValue> | undefined {
    const upper = name.toUpperCase();
    if (SOURCE_CONTROL.has(upper)) return callSourceControl(upper, this.path, args);
    switch (upper) {
      // the designer for what the file is, which is what opening it in the IDE gives
      case 'MODIFY':
        return runtimeUi.openDocument(this.path).then(() => true);
      case 'RUN':
        return runtimeUi.runFile(this.path).then(() => true);
      case 'REMOVE':
        useProjectStore.getState().removeItem(this.item.path);
        return true;
      case 'SETMAIN':
        useProjectStore.getState().setMain(this.item.path);
        return true;
      default:
        void args;
        return undefined;
    }
  }
}

/** The project that is open, as a program sees it. */
export class ProjectObject implements HostObject {
  readonly className = 'Project';

  member(name: string): 'prop' | 'method' | 'none' {
    if (this.get(name) !== undefined) return 'prop';
    return PROJECT_METHODS.has(name.toUpperCase()) ? 'method' : 'none';
  }

  private get doc() {
    return useProjectStore.getState().doc;
  }

  get(name: string): VmValue | HostObject | undefined {
    const store = useProjectStore.getState();
    switch (name.toUpperCase()) {
      case 'NAME':
        return store.path ?? '';
      case 'CLASS':
      case 'BASECLASS':
        return 'Project';
      case 'HOMEDIR': {
        const dir = store.dir();
        return dir ? `${dir}/` : '';
      }
      case 'MAINFILE':
        return this.doc?.main ? store.resolvePath(this.doc.main) : '';
      case 'FILES':
        return new Collection((this.doc?.items ?? []).map((item) => new ProjectFile(item)));
      case 'SERVERS':
        // a server is a class marked OLEPUBLIC, and nothing here is built as one
        return new Collection([]);
      case 'PROJECTHOOK':
        return null;
      case 'SCCPROVIDER':
        return sourceControl().name;
      case 'SCCSTATUS':
        return sourceControl().statusOf(useProjectStore.getState().path ?? '');
      case 'DEBUG':
        return this.doc?.settings?.debugInfo !== false;
      case 'AUTOINCREMENT':
      case 'ENCRYPTED':
        return false;
      case 'BUILDDATETIME':
        return null;
      case 'VERSIONNUMBER':
        return '';
      default:
        return undefined;
    }
  }

  set(name: string, value: VmValue): void {
    if (name.toUpperCase() === 'MAINFILE') {
      this.setMain(typeof value === 'string' ? value : '');
    }
  }

  /** `SetMain(cFileName)`: the file a built application starts at. */
  private setMain(file: string): boolean {
    const store = useProjectStore.getState();
    const named = file.trim();
    if (named === '') {
      store.setMain(undefined);
      return true;
    }
    const wanted = store.relativePath(store.resolvePath(named)).toLowerCase();
    const item = this.doc?.items.find((i) => i.path.toLowerCase() === wanted);
    // only a program or a form can be the one an application starts at
    if (!item || (item.kind !== 'program' && item.kind !== 'form')) return false;
    store.setMain(item.path);
    return true;
  }

  call(name: string, args: VmValue[]): VmValue | Promise<VmValue> | undefined {
    const upper = name.toUpperCase();
    const store = useProjectStore.getState();
    // on the project itself, the six act on the project file - which is the file that says
    // what the project is, and the one VFP puts under control first
    if (SOURCE_CONTROL.has(upper)) return callSourceControl(upper, store.path ?? '', args);
    switch (upper) {
      // Build(cOutputName, nBuildAction, ...): 1 rebuild, 2 an .app, 3 an .exe, 4 a .dll
      case 'BUILD': {
        const [output, action] = args;
        const named = typeof output === 'string' ? output : '';
        const what = typeof action === 'number' ? action : named.toLowerCase().endsWith('.exe') ? 3 : 2;
        if (what === 4) {
          throw new HostError(NO_PROVIDER, 'Build(): a .dll is an in-process server, which this runtime does not build');
        }
        const to = named === '' ? undefined : store.resolvePath(named);
        return (what === 3 ? buildExecutable(to) : buildApp(to)).then((ok) => ok);
      }
      case 'MODIFY':
        return store.path ? runtimeUi.openDocument(store.path).then(() => true) : false;
      case 'RUN': {
        const main = this.doc?.main;
        if (!main) return false;
        return runtimeUi.runFile(store.resolvePath(main)).then(() => true);
      }
      case 'CLOSE':
        store.close();
        return true;
      // SaveAs(cProjectName): the project written somewhere else, and open there
      case 'SAVEAS': {
        const [to] = args;
        if (typeof to !== 'string' || to.trim() === '') return false;
        return saveProjectAs(store.resolvePath(to.trim())).then(() => true);
      }
      case 'SETMAIN':
        return this.setMain(typeof args[0] === 'string' ? args[0] : '');
      // CleanUp(lRemoveObjectCode): a .pjx is a table with records crossed off, and the object
      // code of each file in it. A project here is a file listing what is in it, so there is
      // nothing crossed off to pack and no object code to remove.
      case 'CLEANUP':
        return true;
      default:
        return undefined;
    }
  }
}

/** Writes the open project somewhere else and opens it there, which is what SaveAs does. */
async function saveProjectAs(path: string): Promise<void> {
  const store = useProjectStore.getState();
  const doc = store.doc;
  if (!doc) throw new HostError(NO_PROVIDER, 'SaveAs(): no project is open');
  const from = store.dir();
  const to = dirname(path);
  // every path in a project is relative to the project file, so they move with it
  const items = from === to ? doc.items : doc.items.map((i) => ({ ...i, path: relative(to, join(from ?? to, i.path)) }));
  const name = basename(path).replace(/\.[^.]+$/, '');
  await useProjectStore.getState().saveAs(path, { ...doc, name, items });
}

/** The kind a path is, for a file added to the project by a program. */
export function projectKindFor(path: string): ProjectItem['kind'] {
  return kindForPath(path);
}
