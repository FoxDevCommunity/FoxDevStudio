/**
 * The source-control provider the open project is under.
 *
 * Visual FoxPro asks Windows which SCC provider is registered and talks to that DLL. There is
 * one provider here and it is git: a project that sits inside a working tree is under source
 * control, and one that does not is not. `src/shared/runtime/sourceControl.ts` has the seam and
 * what the mapping does and does not carry across.
 */

import { GitSourceControl, NO_SOURCE_CONTROL, type SourceControlProvider } from '@shared/runtime/sourceControl';
import { getApi } from '../api/foxdev';

let provider: SourceControlProvider = NO_SOURCE_CONTROL;

/** What the project is under now. Never null: a project outside a working tree has a provider
 * that answers "not controlled" to everything. */
export function sourceControl(): SourceControlProvider {
  return provider;
}

/**
 * Looks for a provider around the project's directory, and reads where every file stands.
 * Called when a project is opened, so `SCCStatus` can answer without waiting for git.
 *
 * Nothing here may stop a project from opening. A host that cannot run a command - an older
 * preload, a player window, a test - has no provider, which is a thing a project is allowed to
 * be; and git failing to answer is the same answer.
 */
export async function attachSourceControl(directory: string): Promise<SourceControlProvider> {
  provider = NO_SOURCE_CONTROL;
  const run = getApi().project.capture;
  if (typeof run !== 'function') return provider;
  try {
    const git = new GitSourceControl((command, cwd) => run(command, cwd));
    if (await git.attach(directory)) provider = git;
  } catch {
    // no provider, which is what a project outside source control has anyway
  }
  return provider;
}

/** For tests, and for closing a project: back to having no provider. */
export function setSourceControl(next: SourceControlProvider): void {
  provider = next;
}
