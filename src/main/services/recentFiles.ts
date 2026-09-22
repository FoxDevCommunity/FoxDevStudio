import type { FileService } from './fileService';

export interface RecentFiles {
  list(): Promise<string[]>;
  add(path: string): Promise<void>;
}

/** Most-recent-first list persisted as JSON at `storePath`. */
export function createRecentFiles(fs: FileService, storePath: string, limit = 10): RecentFiles {
  const read = async (): Promise<string[]> => {
    try {
      if (!(await fs.exists(storePath))) return [];
      const parsed: unknown = JSON.parse(await fs.readText(storePath));
      return Array.isArray(parsed) ? parsed.filter((p): p is string => typeof p === 'string') : [];
    } catch {
      return [];
    }
  };
  return {
    list: read,
    async add(path) {
      const next = [path, ...(await read()).filter((p) => p !== path)].slice(0, limit);
      await fs.writeText(storePath, JSON.stringify(next, null, 2));
    },
  };
}
