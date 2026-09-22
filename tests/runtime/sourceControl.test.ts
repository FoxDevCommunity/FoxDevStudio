/**
 * Source control: what git is asked, and what a file's SCCStatus is from what it answers.
 *
 * The provider is driven by a fake command runner, so the test is about the mapping - which
 * git command each of the six methods is, and how `git status --porcelain` becomes the numbers
 * `foxpro.h` names - rather than about git being installed.
 */

import { describe, expect, it } from 'vitest';
import { GitSourceControl, NO_SOURCE_CONTROL, SCC_STATUS } from '@shared/runtime/sourceControl';

const ROOT = 'D:/work/shop';

/** A fake git: answers the two reading commands from what the test sets up, records the rest. */
function fakeGit(tracked: string[], status: string[] = []) {
  const ran: string[] = [];
  const run = async (command: string) => {
    ran.push(command);
    if (command === 'git rev-parse --show-toplevel') return { code: 0, out: `${ROOT}\n`, err: '' };
    if (command === 'git ls-files') return { code: 0, out: tracked.join('\n'), err: '' };
    if (command === 'git status --porcelain') return { code: 0, out: status.join('\n'), err: '' };
    return { code: 0, out: '', err: '' };
  };
  return { ran, scc: new GitSourceControl(run) };
}

describe('a project under git', () => {
  it('is under source control when it sits in a working tree', async () => {
    const { scc } = fakeGit(['orders.prg']);
    await expect(scc.attach(`${ROOT}/src`)).resolves.toBe(true);
    expect(scc.name).toBe('Git');
  });

  it('is not, when git says the directory is not one', async () => {
    const scc = new GitSourceControl(async () => ({ code: 128, out: '', err: 'not a git repository' }));
    await expect(scc.attach('D:/elsewhere')).resolves.toBe(false);
  });

  it('says of each file what foxpro.h calls it', async () => {
    const { scc } = fakeGit(
      ['orders.prg', 'entry.fxf', 'both.prg'],
      [' M orders.prg', 'UU both.prg', '?? scratch.prg'],
    );
    await scc.attach(ROOT);

    // changed in the working tree is what VFP would call checked out to you
    expect(scc.statusOf(`${ROOT}/orders.prg`)).toBe(SCC_STATUS.CHECKED_OUT);
    expect(scc.statusOf(`${ROOT}/entry.fxf`)).toBe(SCC_STATUS.NOT_CHECKED_OUT);
    expect(scc.statusOf(`${ROOT}/both.prg`)).toBe(SCC_STATUS.MERGE_CONFLICT);
    expect(scc.statusOf(`${ROOT}/scratch.prg`)).toBe(SCC_STATUS.NOT_CONTROLLED);
    // outside the working tree is outside source control
    expect(scc.statusOf('D:/elsewhere/other.prg')).toBe(SCC_STATUS.NOT_CONTROLLED);
  });

  it('reads a renamed file under the name it has now', async () => {
    const { scc } = fakeGit(['new.prg'], ['R  old.prg -> new.prg']);
    await scc.attach(ROOT);
    expect(scc.statusOf(`${ROOT}/new.prg`)).toBe(SCC_STATUS.CHECKED_OUT);
  });

  it('runs one git command for each of the six methods, on the path from the top', async () => {
    const { ran, scc } = fakeGit(['orders.prg']);
    await scc.attach(ROOT);
    const file = `${ROOT}/orders.prg`;

    await scc.addToScc(file);
    await scc.removeFromScc(file);
    await scc.checkIn(file, 'Sorted the totals');
    await scc.undoCheckOut(file);
    await scc.getLatestVersion(file);

    expect(ran).toContain('git add -- "orders.prg"');
    expect(ran).toContain('git rm --cached -- "orders.prg"');
    expect(ran).toContain('git commit -m "Sorted the totals" -- "orders.prg"');
    expect(ran).toContain('git checkout -- "orders.prg"');
    expect(ran).toContain('git checkout HEAD -- "orders.prg"');
  });

  it('checks in under a message of its own when the call gives none', async () => {
    const { ran, scc } = fakeGit(['orders.prg']);
    await scc.attach(ROOT);
    await scc.checkIn(`${ROOT}/orders.prg`);
    expect(ran).toContain('git commit -m "Check in orders.prg" -- "orders.prg"');
  });

  it('checks out what git tracks, and nothing it does not', async () => {
    const { scc } = fakeGit(['orders.prg']);
    await scc.attach(ROOT);
    await expect(scc.checkOut(`${ROOT}/orders.prg`)).resolves.toBe(true);
    await expect(scc.checkOut(`${ROOT}/scratch.prg`)).resolves.toBe(false);
  });

  it('answers .F. to everything when there is no provider', async () => {
    const scc = NO_SOURCE_CONTROL;
    expect(scc.name).toBe('');
    expect(scc.statusOf('D:/work/shop/orders.prg')).toBe(SCC_STATUS.NOT_CONTROLLED);
    await expect(scc.addToScc('x')).resolves.toBe(false);
    await expect(scc.checkIn('x')).resolves.toBe(false);
    await expect(scc.getLatestVersion('x')).resolves.toBe(false);
  });

  it('does not do a thing to a file outside the working tree', async () => {
    const { ran, scc } = fakeGit(['orders.prg']);
    await scc.attach(ROOT);
    const before = ran.length;
    await expect(scc.addToScc('D:/elsewhere/other.prg')).resolves.toBe(false);
    expect(ran).toHaveLength(before);
  });
});

describe('a host that cannot run a command', () => {
  it('leaves the project without a provider instead of failing to open it', async () => {
    const { attachSourceControl, sourceControl } = await import('@renderer/runtime/sourceControlProvider');
    // the in-memory api runs nothing, which is what an older preload or a player window is
    await expect(attachSourceControl('D:/work/shop')).resolves.toBeDefined();
    expect(sourceControl().name).toBe('');
    expect(sourceControl().statusOf('D:/work/shop/orders.prg')).toBe(SCC_STATUS.NOT_CONTROLLED);
  });
});
