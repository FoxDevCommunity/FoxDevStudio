/**
 * `HostEvent`: the host waking the runtime with a payload.
 *
 * Everything else the host does either answers at once or answers a request the VM made. A
 * server is neither - it speaks first - so this is the piece that did not exist. It is proved
 * here with something a good deal duller than a server: a program that parks in READ EVENTS
 * with a lambda left where the host can find it, and a host that calls it.
 *
 * The specification is docs/foxscript.md, "Calling back into a VM that is not re-entrant".
 */

import { beforeAll, describe, expect, it } from 'vitest';
import { Scheduler, type SchedulerHooks } from '@shared/runtime/scheduler';
import type { HostRequest, HostRequestHandler, StepResult, VmLike } from '@shared/runtime/host';
import { isFuncRef, type VmValue } from '@shared/runtime/values';
import { compileProgram, createVm } from '@renderer/runtime/vmBridge';
import { loadFoxVm } from '../../src/wasm/foxvm/loader';

// ---- the scheduler's own rules, with a fake VM ------------------------------------------

/** A fake VM driven by a script per fiber, with a table of functions the host may raise. */
class FakeVm implements VmLike {
  private next = 1;
  readonly scripts = new Map<number, StepResult[]>();
  /** Function id -> the fiber starting it produces; a missing id answers null, as the VM does. */
  readonly functions = new Map<number, number>();
  readonly started: { func: number; args: VmValue[] }[] = [];

  addFiber(steps: StepResult[]): number {
    const id = this.next++;
    this.scripts.set(id, [...steps]);
    return id;
  }
  loadModule(): number {
    return 0;
  }
  start(): number {
    throw new Error('use addFiber');
  }
  startMethod(): number | null {
    throw new Error('use addFiber');
  }
  startClassMethod(): number | null {
    throw new Error('use addFiber');
  }
  startFunction(func: number, args: VmValue[]): number | null {
    this.started.push({ func, args });
    return this.functions.get(func) ?? null;
  }
  classDefinitions(): [] {
    return [];
  }
  step(fiber: number): StepResult {
    const script = this.scripts.get(fiber);
    if (!script || script.length === 0) return { state: 'done', value: null, nodefault: false };
    return script.shift()!;
  }
  resume(): void {}
  resumeError(): void {}
  abort(): void {}
  abortAll(): void {}
  callStack(): { program: string; line: number }[] {
    return [];
  }
  setSetting(): void {}
  getSetting(): VmValue {
    return null;
  }
}

const suspend = (request: HostRequest): StepResult => ({ state: 'suspend', request });
const done = (value: VmValue = null): StepResult => ({ state: 'done', value, nodefault: false });

function make(handler: HostRequestHandler, hooks: Partial<SchedulerHooks> = {}) {
  const vm = new FakeVm();
  const onError = hooks.onError ?? (() => 'cancel' as const);
  return { vm, sched: new Scheduler(vm, handler, { ...hooks, onError }) };
}

describe('HostEvent', () => {
  it('runs the lambda straight away when nothing is on the stack', async () => {
    const { vm, sched } = make({ perform: () => null });
    vm.functions.set(7, vm.addFiber([done('ran')]));
    const outcome = await sched.raise({ func: 7, args: ['payload'] });
    expect(outcome?.value).toBe('ran');
    expect(vm.started).toEqual([{ func: 7, args: ['payload'] }]);
  });

  it('answers null for a function the VM no longer has', async () => {
    const { sched } = make({ perform: () => null });
    expect(await sched.raise({ func: 99 })).toBeNull();
  });

  it('queues an event that arrives while a fiber is on the stack, and drains it after', async () => {
    let raised: Promise<{ value: VmValue } | null> | null = null;
    let queuedWhileRunning = -1;
    let startedWhileRunning = -1;
    const { vm, sched } = make({
      perform: () => {
        // a host request is performed with the VM off the stack but a fiber still executing,
        // which is exactly where starting another one would be a re-entrant wasm call
        raised = sched.raise({ func: 7 });
        queuedWhileRunning = sched.queuedEvents;
        startedWhileRunning = vm.started.length;
        return null;
      },
    });
    vm.functions.set(7, vm.addFiber([done('the handler')]));

    await sched.drive(vm.addFiber([suspend({ kind: 'Settle', events: false }), done(null)]));
    expect(queuedWhileRunning).toBe(1);
    expect(startedWhileRunning).toBe(0);
    expect(sched.queuedEvents).toBe(0);
    expect((await raised!)?.value).toBe('the handler');
  });

  it('drains queued events in the order they arrived', async () => {
    const { vm, sched } = make({
      perform: () => {
        void sched.raise({ func: 1 });
        void sched.raise({ func: 2 });
        void sched.raise({ func: 3 });
        return null;
      },
    });
    for (const id of [1, 2, 3]) vm.functions.set(id, vm.addFiber([done(id)]));
    await sched.drive(vm.addFiber([suspend({ kind: 'Settle', events: false }), done(null)]));
    expect(vm.started.map((s) => s.func)).toEqual([1, 2, 3]);
  });

  it('tells a queued event nothing ran when the session is cancelled', async () => {
    let queued: Promise<{ value: VmValue } | null> | null = null;
    const vm = new FakeVm();
    const sched: Scheduler = new Scheduler(
      vm,
      {
        perform: () => {
          queued = sched.raise({ func: 7 });
          sched.cancelAll();
          return null;
        },
      },
      { onError: () => 'cancel' },
    );
    vm.functions.set(7, vm.addFiber([done('never')]));
    const fiber = vm.addFiber([suspend({ kind: 'Settle', events: false }), done(null)]);
    expect(() => sched.drive(fiber)).toThrow('Program cancelled');
    expect(sched.queuedEvents).toBe(0);
    expect(await queued!).toBeNull();
  });
});

// ---- the whole path, with the real VM ----------------------------------------------------

/** The least a VM needs to run a program that prints and parks. */
function reads(output: string[]) {
  return {
    getProp: () => undefined,
    getMember: () => 'none',
    objectClass: () => undefined,
    objectFile: () => undefined,
    output: (text: string) => output.push(text),
    now: () => ({ days: 20000, secs: 0 }),
    random: () => 0.5,
    osInfo: () => ({}),
    mouse: () => ({ row: 0, col: 0, down: false }),
    hasClassMethod: () => false,
    resolveProgram: () => -1,
  };
}

describe('a lambda the host was given and calls back', () => {
  beforeAll(async () => {
    await loadFoxVm();
  });

  it('runs in a fiber of its own while the program is parked in READ EVENTS', async () => {
    const source = [
      'PUBLIC gcSeen, goHandler',
      'gcSeen = ""',
      'goHandler = LAMBDA(cWhat)',
      '  gcSeen = gcSeen + "[" + cWhat + "]"',
      '  RETURN LEN(gcSeen)',
      'ENDLAMBDA',
      '? "parked"',
      'READ EVENTS',
      '',
    ].join('\n');
    const compiled = compileProgram(source, 'main.prg');
    expect(compiled.bytes).toBeTruthy();

    const output: string[] = [];
    const vm = createVm(reads(output));
    const sched = new Scheduler(vm, { perform: () => null }, { onError: () => 'cancel' });
    const module = vm.loadModule(compiled.bytes!);

    // the program parks, so this promise only settles when the session ends
    void sched.runProgram(module);
    expect(output).toContain('parked');

    // the host has the lambda as a plain id, the way it has an object as a handle
    const handed = vm.getGlobal('goHandler');
    expect(isFuncRef(handed)).toBe(true);
    if (!isFuncRef(handed)) throw new Error('unreachable');

    expect((await sched.raise({ func: handed.$fn, args: ['one'] }))?.value).toBe(5);
    expect((await sched.raise({ func: handed.$fn, args: ['two'] }))?.value).toBe(10);
    expect(vm.getGlobal('gcSeen')).toBe('[one][two]');

    sched.cancelAll();
  });

  it('runs one handler to completion before the next, and never re-enters the VM', async () => {
    const source = [
      'PUBLIC gcOrder, goHandler',
      'gcOrder = ""',
      'goHandler = LAMBDA(cWhat)',
      '  gcOrder = gcOrder + "<" + cWhat',
      '  DOEVENTS',
      '  gcOrder = gcOrder + cWhat + ">"',
      'ENDLAMBDA',
      'READ EVENTS',
      '',
    ].join('\n');
    const compiled = compileProgram(source, 'main.prg');
    const output: string[] = [];
    const vm = createVm(reads(output));

    let handler = 0;
    let raisedNested = false;
    let nested: Promise<{ value: VmValue } | null> | null = null;
    const sched: Scheduler = new Scheduler(
      vm,
      {
        // DOEVENTS is performed with the VM off the stack but a fiber still executing: raising
        // here is exactly the case that would trip the bridge's re-entrancy guard without a queue
        perform: (request) => {
          if (request.kind === 'Settle' && !raisedNested) {
            raisedNested = true;
            nested = sched.raise({ func: handler, args: ['inner'] });
          }
          return null;
        },
      },
      { onError: () => 'cancel' },
    );
    const module = vm.loadModule(compiled.bytes!);
    void sched.runProgram(module);

    const handed = vm.getGlobal('goHandler');
    if (!isFuncRef(handed)) throw new Error('the host was not given a lambda');
    handler = handed.$fn;

    await sched.raise({ func: handler, args: ['outer'] });
    await nested!;
    // the inner handler waited: the two never interleaved inside the VM
    expect(vm.getGlobal('gcOrder')).toBe('<outerouter><innerinner>');

    sched.cancelAll();
  });
});
