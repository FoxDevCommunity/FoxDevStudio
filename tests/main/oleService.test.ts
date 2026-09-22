/**
 * The main-process half of the COM bridge: loading the addon, and answering without one.
 *
 * The addon itself is Rust and Windows; this covers what happens around it, including the case
 * that matters most on any other machine - a build with no addon must answer every call rather
 * than throwing, so a program that asks for COM is told and carries on.
 */

import { describe, expect, it } from 'vitest';
import { createOleService } from '@main/services/oleService';

describe('the COM service', () => {
  it('reports COM as unavailable when the addon is not there', () => {
    const service = createOleService(['C:/nowhere/at/all']);
    expect(service.available()).toBe(false);
    expect(service.perform('available', [])).toEqual({ ok: true, value: { kind: 'bool', flag: false } });
  });

  it('answers a call it cannot make with a reason rather than throwing', () => {
    const service = createOleService(['C:/nowhere/at/all']);
    expect(service.perform('create', ['Word.Application'])).toEqual({
      ok: false,
      error: 'COM automation is not available in this build',
    });
  });

  it('loads the addon that was built beside it, on Windows', () => {
    const service = createOleService(['resources/native']);
    if (!service.available()) return; // no addon built here: nothing to check
    const created = service.perform('create', ['Scripting.FileSystemObject']);
    expect(created.ok).toBe(true);
    if (!created.ok) return;

    const handle = created.value.handle ?? 0;
    const base = service.perform('call', [handle, 'GetBaseName', [{ kind: 'string', text: 'C:/dir/report.txt' }]]);
    expect(base).toEqual({ ok: true, value: { kind: 'string', text: 'report' } });

    // a member that answers with an object comes back as a handle of its own
    const drives = service.perform('get', [handle, 'Drives', []]);
    expect(drives.ok && drives.value.kind).toBe('object');
    service.perform('releaseAll', []);
  });
});
