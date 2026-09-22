import { existsSync, readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import { classLibrariesReferenced, importClassLibrary } from '@shared/vfp/importForm';
import { loadClassLibraries } from '@renderer/vfp/classLibraries';
import { setApi } from '@renderer/api/foxdev';
import { createMemoryApi } from '@renderer/api/memoryApi';
import type { DbfReadResult, DbfTableData } from '@shared/vfp/dbfTypes';
import { loadFoxVm, type FoxVmModule } from '../../src/wasm/foxvm/loader';

/**
 * The Visual FoxPro Foundation Classes that ship with FoxDev, read the same way an import reads
 * them. They are the reason a `CLASSLOC` of `_base.vcx` - a file name and nothing else - can be
 * resolved at all.
 */
const FFC = 'resources/ffc';

let vm: FoxVmModule;

function library(stem: string): DbfTableData {
  const bytes = (ext: string) => new Uint8Array(readFileSync(`${FFC}/${stem}.${ext}`));
  const result = vm.read_dbf(bytes('vcx'), bytes('vct')) as DbfReadResult;
  if (!result.ok) throw new Error(`${stem}: ${result.error}`);
  return result;
}

/** Reads a library by whatever path `locate` settled on, which is always inside the FFC folder. */
async function readTable(path: string): Promise<DbfTableData> {
  const stem = path.replace(/\\/g, '/').split('/').pop()!.replace(/\.vcx$/i, '');
  return library(stem);
}

beforeAll(async () => {
  vm = await loadFoxVm();
  // `locate` asks the host whether a candidate path exists; here that question is answered by
  // the real file system, because the libraries being searched for are real files in the repo.
  const api = createMemoryApi();
  api.files.exists = async (path: string) => existsSync(path);
  setApi(api);
});

describe('the bundled Foundation Classes', () => {
  it('are readable, and are the class libraries a real project asks for', () => {
    // a handful that the Solution samples name by hand
    for (const stem of ['_base', '_hyperlink', '_movers', '_datanav', '_menu', '_utility']) {
      const table = library(stem);
      expect(table.records.length, stem).toBeGreaterThan(0);
      expect(table.fields.some((f) => f.name.toUpperCase() === 'CLASSLOC'), stem).toBe(true);
    }
  });

  it('name _base.vcx with no directory, the way Visual FoxPro does', () => {
    // this is the reference no form ever makes and every Foundation Class depends on
    expect(classLibrariesReferenced(library('_hyperlink'))).toEqual(['_base.vcx']);
    expect(classLibrariesReferenced(library('_movers'))).toContain('_base.vcx');
  });

  it("resolve _base.vcx by following a library's own references", async () => {
    const hyperlink = library('_hyperlink');
    const { libraries, missing } = await loadClassLibraries(hyperlink, '/a/form/lives/here', readTable, [FFC]);

    // nothing in the form named _base.vcx; it was reached through _hyperlink.vcx
    expect(missing).toEqual([]);
    expect(libraries.map((l) => l.name.toLowerCase())).toContain('_base.vcx');
  });

  it('let a class built on one resolve, instead of arriving empty', async () => {
    const hyperlink = library('_hyperlink');
    const { libraries } = await loadClassLibraries(hyperlink, '/a/form/lives/here', readTable, [FFC]);
    const classes = importClassLibrary(hyperlink, '_hyperlink', libraries);

    expect(classes.length).toBeGreaterThan(0);
    const unresolved = classes.flatMap((c) => c.imported.warnings).filter((w) => w.kind === 'classNotFound');
    expect(unresolved.map((w) => w.message)).toEqual([]);
  });

  it('stop rather than loop when libraries refer to each other', async () => {
    // _movers.vcx names itself as well as _base.vcx, which a naive walk would follow for ever
    const { libraries } = await loadClassLibraries(library('_movers'), '/elsewhere', readTable, [FFC]);
    const names = libraries.map((l) => l.name.toLowerCase());
    expect(new Set(names).size).toBe(names.length);
  });
});
