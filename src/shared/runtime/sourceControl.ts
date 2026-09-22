/**
 * Source control, as a project and its files reach it.
 *
 * Visual FoxPro does not do version control itself. It talks to an **SCC provider** - a DLL
 * implementing Microsoft's Source Code Control interface, which in 2004 meant Visual SourceSafe
 * - and the six methods on the Project and File objects (`AddToSCC`, `CheckIn`, `CheckOut`,
 * `GetLatestVersion`, `RemoveFromSCC`, `UndoCheckOut`) are that interface seen from a program.
 *
 * This is the seam, and one provider behind it: git. The mapping is not exact, and the places it
 * is not are written down rather than papered over:
 *
 * - **Check-out is not a lock.** VFP's model is file-level check-out with exclusive locks, so
 *   `SCCStatus` can say "checked out to someone else". Git has nothing of the kind: a working
 *   copy is always writable and nobody else can see that you have edited a file. So `CheckOut`
 *   only says whether the file is there to edit, and the two statuses about other users - 3 and
 *   6 - never come back.
 * - **Modified is checked out.** A file you have changed in the working tree is what VFP would
 *   call checked out to you, so that is status 2.
 * - **The latest version is HEAD.** `GetLatestVersion` restores the file from the last commit.
 *   It does not go to a remote: fetching over the network inside a property read is not
 *   something a program calling this expects to wait for.
 * - **Status 5, merged without conflict, is never returned.** Git does not keep the fact that a
 *   file merged cleanly once the merge is over, and inventing it would make the number a guess.
 */

/** What `SCCStatus` answers, as `foxpro.h` names the values. */
export const SCC_STATUS = {
  /** SCCFILE_NOTCONTROLLED */
  NOT_CONTROLLED: 0,
  /** SCCFILE_NOTCHECKEDOUT */
  NOT_CHECKED_OUT: 1,
  /** SCCFILE_CHECKEDOUTCU - checked out to the current user */
  CHECKED_OUT: 2,
  /** SCCFILE_CHECKEDOUTOU - checked out to someone else; git has no locks, so never this */
  CHECKED_OUT_OTHER: 3,
  /** SCCFILE_MERGECONFLICT */
  MERGE_CONFLICT: 4,
  /** SCCFILE_MERGE - merged without conflict; not something git still knows afterwards */
  MERGED: 5,
  /** SCCFILE_CHECKEDOUTMU - checked out to several users; git has no locks, so never this */
  CHECKED_OUT_MANY: 6,
} as const;

/** Runs a command line and hands back what it wrote. */
export type RunCommand = (command: string, cwd: string) => Promise<{ code: number; out: string; err: string }>;

export interface SourceControlProvider {
  /** What `SCCProvider` answers with, or the empty string when the project is not controlled. */
  readonly name: string;
  /** Reads where the project stands. Answers whether it is under source control at all. */
  attach(directory: string): Promise<boolean>;
  /** Reads the state of every file again, which every method that changes something does. */
  refresh(): Promise<void>;
  /** `SCCStatus` for one file, from what the last refresh read. */
  statusOf(path: string): number;
  addToScc(path: string): Promise<boolean>;
  removeFromScc(path: string): Promise<boolean>;
  checkOut(path: string): Promise<boolean>;
  checkIn(path: string, comment?: string): Promise<boolean>;
  undoCheckOut(path: string): Promise<boolean>;
  getLatestVersion(path: string): Promise<boolean>;
}

/** The two-letter codes `git status --porcelain` gives a file both sides of a merge touched. */
const CONFLICTED = new Set(['DD', 'AU', 'UD', 'UA', 'DU', 'AA', 'UU']);

/** Lower-cased, forward-slashed, with no trailing slash: how a path is compared here. */
function key(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase();
}

/** A path with quotes around it, so a directory with a space in it still reaches git. */
function quoted(path: string): string {
  return `"${path.replace(/"/g, '\\"')}"`;
}

/**
 * Git behind the source-control seam.
 *
 * Everything it knows comes from two commands - what is tracked, and what has changed - read
 * again after anything that changes them, so `SCCStatus` can answer without waiting.
 */
export class GitSourceControl implements SourceControlProvider {
  readonly name = 'Git';

  /** The top of the working tree, which is what every path is relative to. */
  private root = '';
  private tracked = new Set<string>();
  private changed = new Set<string>();
  private conflicted = new Set<string>();

  constructor(private readonly run: RunCommand) {}

  async attach(directory: string): Promise<boolean> {
    this.root = '';
    this.tracked.clear();
    this.changed.clear();
    this.conflicted.clear();
    if (!directory) return false;
    const top = await this.run('git rev-parse --show-toplevel', directory);
    if (top.code !== 0) return false;
    this.root = top.out.trim().replace(/\\/g, '/');
    if (!this.root) return false;
    await this.refresh();
    return true;
  }

  async refresh(): Promise<void> {
    if (!this.root) return;
    this.tracked = new Set();
    this.changed = new Set();
    this.conflicted = new Set();

    const listed = await this.run('git ls-files', this.root);
    if (listed.code === 0) {
      for (const line of listed.out.split(/\r?\n/)) {
        if (line.trim()) this.tracked.add(key(line.trim()));
      }
    }

    const status = await this.run('git status --porcelain', this.root);
    if (status.code !== 0) return;
    for (const line of status.out.split(/\r?\n/)) {
      if (line.length < 4) continue;
      const code = line.slice(0, 2);
      // a rename is written `R  old -> new`, and it is the new name that is the file now
      const named = line.slice(3).trim();
      const path = key((named.split(' -> ').pop() ?? named).replace(/^"|"$/g, ''));
      if (CONFLICTED.has(code)) this.conflicted.add(path);
      else if (code !== '??') this.changed.add(path);
    }
  }

  statusOf(path: string): number {
    const relative = this.relative(path);
    if (!relative) return SCC_STATUS.NOT_CONTROLLED;
    if (this.conflicted.has(relative)) return SCC_STATUS.MERGE_CONFLICT;
    if (!this.tracked.has(relative)) return SCC_STATUS.NOT_CONTROLLED;
    return this.changed.has(relative) ? SCC_STATUS.CHECKED_OUT : SCC_STATUS.NOT_CHECKED_OUT;
  }

  async addToScc(path: string): Promise<boolean> {
    return this.act(path, (file) => `git add -- ${quoted(file)}`);
  }

  /** Stops tracking the file without deleting it, which is what leaving source control means. */
  async removeFromScc(path: string): Promise<boolean> {
    return this.act(path, (file) => `git rm --cached -- ${quoted(file)}`);
  }

  /**
   * There is nothing to take a lock on, so this says whether the file is there to be edited:
   * true when git is tracking it, false when it is not under control at all.
   */
  async checkOut(path: string): Promise<boolean> {
    const relative = this.relative(path);
    return !!relative && this.tracked.has(relative);
  }

  async checkIn(path: string, comment?: string): Promise<boolean> {
    const relative = this.relative(path);
    if (!relative) return false;
    const added = await this.run(`git add -- ${quoted(relative)}`, this.root);
    if (added.code !== 0) return false;
    const message = comment?.trim() || `Check in ${relative}`;
    const committed = await this.run(`git commit -m ${quoted(message)} -- ${quoted(relative)}`, this.root);
    await this.refresh();
    return committed.code === 0;
  }

  /** Throws the working copy's changes away, which is what undoing a check-out does. */
  async undoCheckOut(path: string): Promise<boolean> {
    return this.act(path, (file) => `git checkout -- ${quoted(file)}`);
  }

  /** The file as the last commit has it. VFP would leave it read-only; git's is writable. */
  async getLatestVersion(path: string): Promise<boolean> {
    return this.act(path, (file) => `git checkout HEAD -- ${quoted(file)}`);
  }

  /** Runs one command against a file inside the tree, then reads the state again. */
  private async act(path: string, command: (relative: string) => string): Promise<boolean> {
    const relative = this.relative(path);
    if (!relative) return false;
    const result = await this.run(command(relative), this.root);
    await this.refresh();
    return result.code === 0;
  }

  /** The file's path from the top of the working tree, or empty when it is outside it. */
  private relative(path: string): string {
    if (!this.root || !path) return '';
    const full = key(path);
    const root = key(this.root);
    if (full === root) return '';
    if (!full.startsWith(`${root}/`)) return '';
    return full.slice(root.length + 1);
  }
}

/** A project that is not under source control: what every method answers before there is one. */
export const NO_SOURCE_CONTROL: SourceControlProvider = {
  name: '',
  attach: async () => false,
  refresh: async () => {},
  statusOf: () => SCC_STATUS.NOT_CONTROLLED,
  addToScc: async () => false,
  removeFromScc: async () => false,
  checkOut: async () => false,
  checkIn: async () => false,
  undoCheckOut: async () => false,
  getLatestVersion: async () => false,
};
