import { join } from '@shared/paths';
import { createEmptyProjectDocument, parseProjectDocument, stringifyProjectDocument } from '@shared/project/serialize';
import type { ProjectDocument } from '@shared/project/schema';
import type { ProjectHandle } from '@shared/ipc/api';
import type { FileService } from './fileService';

export function createProjectService(fs: FileService) {
  return {
    async create(dir: string, name: string): Promise<ProjectHandle> {
      await fs.mkdirp(dir);
      const path = join(dir, `${name}.fxproject`);
      const doc = createEmptyProjectDocument(name);
      await fs.writeText(path, stringifyProjectDocument(doc));
      return { path, doc };
    },
    async open(path: string): Promise<ProjectHandle> {
      const r = parseProjectDocument(await fs.readText(path));
      if (!r.ok) throw new Error(`Cannot open project: ${r.error}`);
      return { path, doc: r.doc };
    },
    async save(path: string, doc: ProjectDocument): Promise<void> {
      await fs.writeText(path, stringifyProjectDocument(doc));
    },
  };
}
