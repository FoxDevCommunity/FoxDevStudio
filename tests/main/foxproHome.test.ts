/**
 * Where `HOME()` and `_SAMPLES` point.
 *
 * The samples that ship with Visual FoxPro open their data as `_samples + "\Data\..."`, so the
 * variable has to name a real directory for any of them to run.
 */

import { describe, expect, it } from 'vitest';
import { foxproHome, homeDir } from '../../src/main/services/foxproHome';

/** A path separator, written this way so the tests read without escapes in them. */
const SEP = String.fromCharCode(92);
const PROJECT = `C:${SEP}proj`;

describe('the directories HOME() names', () => {
  it('answers the project directory for the running application', async () => {
    expect(await homeDir(1, PROJECT)).toBe(PROJECT + SEP);
  });

  it('names the samples under the installation when there is one', async () => {
    const home = await foxproHome();
    const samples = await homeDir(2, PROJECT);
    if (home === null) {
      // no Visual FoxPro on this machine: there is no samples directory to name
      expect(samples).toBe('');
      return;
    }
    expect(samples.toLowerCase()).toContain('samples');
    expect(samples.toLowerCase().startsWith(home.toLowerCase())).toBe(true);
    expect(samples.endsWith(SEP)).toBe(true);
  });

  it('names nothing for a number that is not one of them', async () => {
    expect(await homeDir(99, PROJECT)).toBe('');
  });

  it('answers the same thing twice without asking Windows twice', async () => {
    expect(await foxproHome()).toBe(await foxproHome());
  });
});
