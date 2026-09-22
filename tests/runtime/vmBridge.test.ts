import { beforeAll, describe, expect, it } from 'vitest';
import { Scheduler } from '@shared/runtime/scheduler';
import type { HostReads, HostRequest } from '@shared/runtime/host';
import type { VmValue } from '@shared/runtime/values';
import { compileError, compileForm, compileProgram, compileSnippet, createVm, ReentrancyError, type WasmVm } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

/** Minimal object tree standing in for the Desktop: a form with a text box and a label. */
class StubHost implements HostReads {
  readonly lines: string[] = [];
  readonly props = new Map<string, VmValue>();
  private readonly children = new Map<string, number>();
  private readonly classes = new Map<number, string>();

  constructor() {
    this.classes.set(1, 'Form');
    this.classes.set(2, 'TextBox');
    this.classes.set(3, 'Label');
    this.children.set('1:txtName', 2);
    this.children.set('1:lblGreeting', 3);
    this.props.set('2:Value', 'ada');
    this.props.set('3:Caption', '(nothing yet)');
    this.props.set('1:Caption', 'Hello, World');
  }

  key(obj: number, name: string): string {
    return `${obj}:${name}`;
  }
  getProp(obj: number, name: string): VmValue | undefined {
    if (name.toLowerCase() === 'parent') return obj === 1 ? null : { $obj: 1 };
    if (name.toLowerCase() === 'name') return obj === 1 ? 'frmHello' : 'child';
    return this.props.get(this.key(obj, name));
  }
  getMember(obj: number, name: string): number | 'prop' | 'method' | 'none' {
    const child = this.children.get(`${obj}:${name}`);
    if (child !== undefined) return child;
    if (this.props.has(this.key(obj, name))) return 'prop';
    if (['parent', 'name'].includes(name.toLowerCase())) return 'prop';
    if (['setfocus', 'refresh', 'release'].includes(name.toLowerCase())) return 'method';
    return 'none';
  }
  objectClass(obj: number): string | null {
    return this.classes.get(obj) ?? null;
  }
  objectFile(): string | null {
    return null;
  }
  output(text: string, newline: boolean): void {
    if (newline || this.lines.length === 0) this.lines.push(text);
    else this.lines[this.lines.length - 1] += text;
  }
  now(): { days: number; secs: number } {
    return { days: 20704, secs: 47107 }; // 2026-09-07 13:05:07
  }
  random(): number {
    return 0.5;
  }
  osInfo(): string {
    return 'Windows|10|0|26200';
  }
  hasClassMethod(): boolean {
    return false;
  }
  resolveProgram(): number {
    return -1;
  }
}

function makeSession(host: StubHost, perform?: (r: HostRequest) => VmValue | Promise<VmValue>) {
  const vm = createVm(host);
  const requests: HostRequest[] = [];
  const scheduler = new Scheduler(
    vm,
    {
      perform(request) {
        requests.push(request);
        if (perform) return perform(request);
        if (request.kind === 'SetProp') {
          host.props.set(host.key(request.obj, request.name), request.value);
          return null;
        }
        return null;
      },
    },
    { onError: () => 'cancel' },
  );
  return { vm, scheduler, requests };
}

beforeAll(async () => {
  await loadFoxVm();
});

describe('FoxVM bridge', () => {
  it('compiles and runs a program, sending output to the host', () => {
    const host = new StubHost();
    const { vm, scheduler } = makeSession(host);
    const out = compileProgram('LOCAL n\nn = 6\n? "sum:", n * 7\n? UPPER("done")', 'demo');
    expect(compileError(out)).toBeNull();
    const module = vm.loadModule(out.bytes!);
    scheduler.runProgram(module);
    expect(host.lines).toEqual(['sum:           42', 'DONE']);
  });

  it('reports compile errors instead of a module', () => {
    const out = compileProgram('IF x\n', 'broken');
    expect(out.bytes).toBeNull();
    expect(compileError(out)).toMatch(/ENDIF/);
  });

  it('runs a form method against the host object tree', () => {
    const host = new StubHost();
    const { vm, scheduler, requests } = makeSession(host);
    const out = compileForm('HelloWorld', [
      {
        objectPath: 'cmdSayHi',
        event: 'Click',
        params: '',
        source: 'LOCAL cMsg\ncMsg = "Hello, " + PROPER(ALLTRIM(THISFORM.txtName.Value))\nTHISFORM.lblGreeting.Caption = cMsg',
      },
    ]);
    expect(compileError(out)).toBeNull();
    const module = vm.loadModule(out.bytes!);

    const outcome = scheduler.dispatch(module, 'cmdSayHi', 'Click', 2);
    expect(outcome).not.toBeNull();
    expect(host.props.get('3:Caption')).toBe('Hello, Ada');
    expect(requests.map((r) => r.kind)).toEqual(['SetProp']);
  });

  it('suspends on MESSAGEBOX and resumes with the pressed button', async () => {
    const host = new StubHost();
    const { vm, scheduler, requests } = makeSession(host, (r) => (r.kind === 'MessageBox' ? Promise.resolve(7) : null));
    const out = compileSnippet('IF MESSAGEBOX("Save?", 4, "FoxDev") = 6\n  ? "saved"\nELSE\n  ? "discarded"\nENDIF', 'cmd');
    const module = vm.loadModule(out.bytes!);

    const result = scheduler.runProgram(module);
    expect(result).toBeInstanceOf(Promise);
    await result;
    expect(requests[0]).toMatchObject({ kind: 'MessageBox', text: 'Save?', flags: 4, title: 'FoxDev' });
    expect(host.lines).toEqual(['discarded']);
  });

  it('surfaces runtime errors with the program and line', () => {
    const host = new StubHost();
    const vm = createVm(host);
    const errors: { program: string; line: number; message: string }[] = [];
    const scheduler = new Scheduler(
      vm,
      { perform: () => null },
      {
        onError: (e) => {
          errors.push({ program: e.program, line: e.line, message: e.message });
          return 'cancel';
        },
      },
    );
    const out = compileProgram('? "start"\n? nope + 1\n', 'oops');
    const module = vm.loadModule(out.bytes!);
    expect(() => scheduler.runProgram(module)).toThrow();
    expect(errors).toEqual([{ program: 'OOPS', line: 2, message: "Variable 'NOPE' is not found." }]);
  });

  it('performs side effects with the VM off the stack, so nested dispatch is safe', () => {
    const host = new StubHost();
    const vm = createVm(host);
    const nested: string[] = [];
    const scheduler: Scheduler = new Scheduler(
      vm,
      {
        perform(request) {
          if (request.kind === 'SetProp') {
            host.props.set(host.key(request.obj, request.name), request.value);
            // VFP fires ProgrammaticChange from inside the assignment; it must be able to run now
            const outcome = scheduler.dispatch(module, 'txtName', 'InteractiveChange', 2);
            nested.push(outcome === null ? 'no handler' : 'ran');
          }
          return null;
        },
      },
      { onError: () => 'cancel' },
    );
    const out = compileForm('F', [
      { objectPath: 'txtName', event: 'Click', params: '', source: 'THISFORM.lblGreeting.Caption = "x"' },
      { objectPath: 'txtName', event: 'InteractiveChange', params: '', source: '? "changed"' },
    ]);
    const module = vm.loadModule(out.bytes!);

    scheduler.dispatch(module, 'txtName', 'Click', 2);
    expect(nested).toEqual(['ran']);
    expect(host.lines).toEqual(['changed']);
    expect(host.props.get('3:Caption')).toBe('x');
  });

  it('refuses a re-entrant call from a synchronous host read', () => {
    const host = new StubHost();
    // the trap needs the VM and the VM needs the trap, so hold it in a box
    const box: { vm?: WasmVm } = {};
    // getProp runs *inside* wasm, so calling the VM from it is the real hazard
    const trap: HostReads = {
      getProp: (obj, name) => (name === 'Caption' ? box.vm!.getSetting('exact') : host.getProp(obj, name)),
      getMember: (obj, name) => host.getMember(obj, name),
      objectClass: (obj) => host.objectClass(obj),
      objectFile: () => null,
      output: (text, newline) => host.output(text, newline),
      now: () => host.now(),
      random: () => host.random(),
      osInfo: () => host.osInfo(),
      hasClassMethod: () => host.hasClassMethod(),
      resolveProgram: () => host.resolveProgram(),
    };
    const vm = (box.vm = createVm(trap));
    const out = compileForm('F', [{ objectPath: 'txtName', event: 'Click', params: '', source: '? THISFORM.lblGreeting.Caption' }]);
    const module = vm.loadModule(out.bytes!);
    const scheduler = new Scheduler(vm, { perform: () => null }, { onError: () => 'cancel' });
    expect(() => scheduler.dispatch(module, 'txtName', 'Click', 2)).toThrow(ReentrancyError);
  });
});

it('does not let a throwing host read poison the VM', () => {
  // A host read runs inside wasm. An exception thrown out of one unwinds the wasm frame without
  // unwinding the Rust one, and the VM stays borrowed: every call after it dies with "recursive
  // use of an object detected" and the session is gone. The read answers nothing instead, and
  // the reason is raised once wasm is off the stack.
  const host = new StubHost();
  const trap: HostReads = {
    getProp: (obj, name) => {
      if (name === 'Caption') throw new Error('a read that misbehaved');
      return host.getProp(obj, name);
    },
    getMember: (obj, name) => host.getMember(obj, name),
    objectClass: (obj) => host.objectClass(obj),
    objectFile: () => null,
    output: (text, newline) => host.output(text, newline),
    now: () => host.now(),
    random: () => host.random(),
    osInfo: () => host.osInfo(),
    hasClassMethod: () => host.hasClassMethod(),
    resolveProgram: () => host.resolveProgram(),
  };
  const vm = createVm(trap);
  const out = compileForm('F', [{ objectPath: 'txtName', event: 'Click', params: '', source: '? THISFORM.lblGreeting.Caption' }]);
  const module = vm.loadModule(out.bytes!);
  const scheduler = new Scheduler(vm, { perform: () => null }, { onError: () => 'cancel' });

  expect(() => scheduler.dispatch(module, 'txtName', 'Click', 2)).toThrow('a read that misbehaved');
  // and the VM still works afterwards, which is the whole point
  expect(vm.getSetting('exact')).toBeDefined();
});
