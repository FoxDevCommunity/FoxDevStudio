import { dirname, isInside, normalize } from '@shared/paths';

/**
 * The renderer may only read/write under directories it obtained legitimately:
 * paths returned by native dialogs, the open project's directory, and userData.
 */
export class PathGuard {
  /** Directories obtained directly: a dialog result, the open project, userData. */
  private roots = new Set<string>();
  /** Directories reached out to from a root by `allowNeighbour`. */
  private derived = new Set<string>();

  allowDir(dir: string): void {
    this.roots.add(normalize(dir));
  }

  allowFile(path: string): void {
    this.allowDir(dirname(path));
  }

  /**
   * Grants the folder holding `path` when it sits near something obtained directly: inside an
   * ancestor no more than `levels` steps above one of the roots.
   *
   * A Visual FoxPro project reaches sideways out of its own folder - the Solution samples refer
   * to `..\classes` beside them and to the `ffc` folder at the product root - so importing one
   * needs more than the directory the project file sits in. Two levels is what reaches the
   * product root from a samples folder.
   *
   * What is granted this way never becomes a root itself, so reaching out cannot be repeated to
   * walk anywhere: every grant is still measured from a directory the user chose.
   */
  allowNeighbour(path: string, levels = 2): boolean {
    for (const root of this.roots) {
      let ancestor = root;
      for (let i = 0; i <= levels; i++) {
        if (isInside(ancestor, path)) {
          this.derived.add(normalize(dirname(path)));
          return true;
        }
        const up = dirname(ancestor);
        if (up === ancestor) break;
        ancestor = up;
      }
    }
    return false;
  }

  isAllowed(path: string): boolean {
    for (const root of this.roots) if (isInside(root, path)) return true;
    for (const root of this.derived) if (isInside(root, path)) return true;
    return false;
  }

  check(path: string): void {
    if (!this.isAllowed(path)) throw new Error(`Access denied: ${path}`);
  }
}
