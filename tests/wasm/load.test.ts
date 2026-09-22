import { readFileSync } from 'node:fs';
import { loadFoxVm, loadFoxVmSync } from '../../src/wasm/foxvm/loader';

describe('foxvm wasm module', () => {
  it('loads and reports the crate version', async () => {
    const vm = await loadFoxVm();
    const cargo = readFileSync('crates/foxvm/Cargo.toml', 'utf8');
    const crateVersion = /^version = "([^"]+)"/m.exec(cargo)?.[1];
    expect(crateVersion).toBeTruthy();
    expect(vm.version()).toBe(crateVersion);
  });

  it('calls into wasm', () => {
    const result = loadFoxVmSync().check('? 1 + 1', 'program') as { diagnostics: unknown[] };
    expect(result.diagnostics).toEqual([]);
  });
});
