import { describe, expect, it, vi } from 'vitest';
import { Cancelled, Scheduler, type SchedulerHooks } from '@shared/runtime/scheduler';
import { HostError, type BreakReason, type BreakStop, type HostRequest, type HostRequestHandler, type StepMode, type StepResult, type VmLike } from '@shared/runtime/host';
import type { VmValue } from '@shared/runtime/values';

/**
 * A fake VM driven by a script per fiber: each entry is what the next `step` returns, and a
 * `resume` records the value the scheduler fed back.
 */
class FakeVm implements VmLike {
  private next = 1;
  readonly scripts = new Map<number, StepResult[]>();
  readonly resumed = new Map<number, VmValue[]>();
  readonly aborted: number[] = [];
  readonly errors: { fiber: number; code: number; message: string }[] = [];

  addFiber(steps: StepResult[]): number {
    const id = this.next++;
    this.scripts.set(id, [...steps]);
    this.resumed.set(id, []);
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
  startFunction(): number | null {
    throw new Error('use addFiber');
  }
  classDefinitions(): [] {
    return [];
  }
  step(fiber: number): StepResult {
    const script = this.scripts.get(fiber);
    if (!script || script.length === 0) return { state: 'done', value: null, nodefault: false };
    return script.shift()!;
  }
  resume(fiber: number, value: VmValue): void {
    this.resumed.get(fiber)?.push(value);
  }
  resumeError(fiber: number, code: number, message: string): void {
    this.errors.push({ fiber, code, message });
  }
  abort(fiber: number): void {
    this.aborted.push(fiber);
  }
  abortAll(): void {
    this.aborted.push(-1);
  }
  callStack(): { program: string; line: number }[] {
    return [];
  }
  setSetting(): void {}
  getSetting(): VmValue {
    return null;
  }
  readonly stepModes: { fiber: number; mode: StepMode }[] = [];
  setStepMode(fiber: number, mode: StepMode): void {
    this.stepModes.push({ fiber, mode });
  }
}

const suspend = (request: HostRequest): StepResult => ({ state: 'suspend', request });
const done = (value: VmValue = null, nodefault = false): StepResult => ({ state: 'done', value, nodefault });
const setProp = (obj: number, name: string, value: VmValue): HostRequest => ({ kind: 'SetProp', obj, name, value });

function make(handler: HostRequestHandler, hooks: Partial<SchedulerHooks> = {}) {
  const vm = new FakeVm();
  const onError = hooks.onError ?? (() => 'cancel' as const);
  const sched = new Scheduler(vm, handler, { ...hooks, onError });
  return { vm, sched };
}

describe('Scheduler', () => {
  it('performs synchronous requests without leaving the tick', () => {
    const performed: HostRequest[] = [];
    const { vm, sched } = make({
      perform(req) {
        performed.push(req);
        return 42;
      },
    });
    const fiber = vm.addFiber([suspend(setProp(1, 'Caption', 'x')), suspend(setProp(1, 'Top', 5)), done('finished')]);

    const result = sched.drive(fiber);
    expect(result).not.toBeInstanceOf(Promise);
    expect((result as { value: VmValue }).value).toBe('finished');
    expect(performed).toHaveLength(2);
    expect(vm.resumed.get(fiber)).toEqual([42, 42]);
    expect(sched.currentState).toBe('idle');
  });

  it('awaits a promised request and reports waiting while parked', async () => {
    let release!: (v: VmValue) => void;
    const { vm, sched } = make({
      perform: () => new Promise<VmValue>((r) => (release = r)),
    });
    const fiber = vm.addFiber([suspend({ kind: 'MessageBox', text: 'ok?', flags: 4, title: 'T', timeout: null }), done('after')]);

    const pending = sched.drive(fiber) as Promise<{ value: VmValue }>;
    expect(pending).toBeInstanceOf(Promise);
    expect(sched.currentState).toBe('waiting');

    release(7);
    expect((await pending).value).toBe('after');
    expect(vm.resumed.get(fiber)).toEqual([7]);
    expect(sched.currentState).toBe('idle');
  });

  it('runs a nested dispatch inside perform before the outer fiber resumes', () => {
    const order: string[] = [];
    const vm = new FakeVm();
    const outer = vm.addFiber([suspend(setProp(1, 'Value', 'typed')), done('outer done')]);
    const inner = vm.addFiber([done('inner done')]);
    const sched = new Scheduler(
      vm,
      {
        perform() {
          order.push('perform start');
          const nested = sched.drive(inner) as { value: VmValue };
          order.push(`nested ${String(nested.value)}`);
          return null;
        },
      },
      { onError: () => 'cancel' },
    );

    const result = sched.drive(outer) as { value: VmValue };
    order.push(`outer ${String(result.value)}`);
    expect(order).toEqual(['perform start', 'nested inner done', 'outer outer done']);
  });

  it('parks READ EVENTS until another fiber runs CLEAR EVENTS', async () => {
    const { vm, sched } = make({ perform: () => null });
    const main = vm.addFiber([suspend({ kind: 'ReadEvents' }), done('main ended')]);
    const menu = vm.addFiber([suspend({ kind: 'ClearEvents' }), done('menu ended')]);

    const mainPending = sched.drive(main) as Promise<{ value: VmValue }>;
    expect(sched.currentState).toBe('waiting');
    expect(sched.readEventsDepth).toBe(1);

    // an event handler runs while the program is parked, and asks it to end
    const menuResult = sched.drive(menu);
    expect((menuResult as { value: VmValue }).value).toBe('menu ended');

    expect((await mainPending).value).toBe('main ended');
    expect(sched.readEventsDepth).toBe(0);
  });

  it('turns a thrown HostError into a catchable VFP error', () => {
    const { vm, sched } = make({
      perform() {
        throw new HostError(1, "File 'missing.txt' does not exist");
      },
    });
    const fiber = vm.addFiber([suspend({ kind: 'FileRead', path: 'missing.txt', search: [] }), done()]);
    sched.drive(fiber);
    expect(vm.errors).toEqual([{ fiber, code: 1, message: "File 'missing.txt' does not exist" }]);
  });

  it('ignores an error by resuming, or cancels everything', () => {
    const error = { code: 12, message: "Variable 'X' is not found", program: 'main', line: 3 };
    const ignored = make({ perform: () => null }, { onError: () => 'ignore' });
    const f1 = ignored.vm.addFiber([{ state: 'error', error, stack: [] }, done('continued')]);
    expect((ignored.sched.drive(f1) as { value: VmValue }).value).toBe('continued');

    const cancelled = make({ perform: () => null }, { onError: () => 'cancel' });
    const f2 = cancelled.vm.addFiber([{ state: 'error', error, stack: [] }, done()]);
    expect(() => cancelled.sched.drive(f2)).toThrow(Cancelled);
    expect(cancelled.vm.aborted).toContain(-1);
  });

  it('does not resume a fiber whose session was cancelled', async () => {
    let release!: (v: VmValue) => void;
    const { vm, sched } = make({ perform: () => new Promise<VmValue>((r) => (release = r)) });
    const fiber = vm.addFiber([suspend({ kind: 'InputBox', prompt: 'name', title: '', default: '', timeout: null, timeout_value: '' }), done()]);

    const pending = (sched.drive(fiber) as Promise<unknown>).catch((e: unknown) => e);
    sched.cancelAll();
    release('too late');
    await expect(pending).resolves.toBeInstanceOf(Cancelled);
    expect(vm.resumed.get(fiber)).toEqual([]);
  });

  it('reports state transitions and idle', async () => {
    const states: string[] = [];
    const onIdle = vi.fn();
    let release!: (v: VmValue) => void;
    const { vm, sched } = make(
      { perform: () => new Promise<VmValue>((r) => (release = r)) },
      { onStateChange: (s) => states.push(s), onIdle },
    );
    const fiber = vm.addFiber([suspend({ kind: 'WaitWindow', text: 'hi', nowait: false, timeout: null, clear: false }), done()]);
    const pending = sched.drive(fiber) as Promise<unknown>;
    release(null);
    await pending;
    expect(states).toEqual(['running', 'waiting', 'running', 'idle']);
    expect(onIdle).toHaveBeenCalled();
  });

  describe('the debugger', () => {
    const breakAt = (program: string, line: number, reason: BreakReason = 'breakpoint'): HostRequest => ({
      kind: 'Break',
      program,
      line,
      reason,
    });

    it('parks a program that stops and lets it go with the step the developer asked for', async () => {
      const stops: BreakStop[] = [];
      const resumed: number[] = [];
      const { vm, sched } = make(
        { perform: () => null },
        { onBreak: (s) => stops.push(s), onResumed: (f) => resumed.push(f) },
      );
      const fiber = vm.addFiber([suspend(breakAt('counting', 4)), suspend(breakAt('counting', 5, 'step')), done('ran on')]);

      const pending = sched.drive(fiber) as Promise<{ value: VmValue }>;
      expect(pending).toBeInstanceOf(Promise);
      expect(stops).toEqual([{ fiber, program: 'counting', line: 4, reason: 'breakpoint' }]);
      expect(sched.currentState).toBe('waiting');
      // the fiber is parked, so nothing has been resumed and the IDE is free
      expect(vm.resumed.get(fiber)).toEqual([]);

      sched.letGo(fiber, 'into');
      // the step is set before the resume, while the VM is off the stack
      expect(vm.stepModes).toEqual([{ fiber, mode: 'into' }]);
      await Promise.resolve();
      expect(stops).toHaveLength(2);
      expect(stops[1]?.reason).toBe('step');

      sched.letGo(fiber);
      expect((await pending).value).toBe('ran on');
      expect(vm.stepModes[1]).toEqual({ fiber, mode: 'go' });
      expect(resumed).toEqual([fiber, fiber]);
      expect(sched.currentState).toBe('idle');
    });

    it('lets RESUME in another fiber go the program that is stopped', async () => {
      const { vm, sched } = make({ perform: () => null });
      const stopped = vm.addFiber([suspend(breakAt('main', 3, 'suspend')), done('carried on')]);
      const command = vm.addFiber([suspend({ kind: 'DebugResume' }), done('typed')]);

      const pending = sched.drive(stopped) as Promise<{ value: VmValue }>;
      expect(sched.stops.map((s) => s.line)).toEqual([3]);

      // RESUME is typed in the Command Window, which is a fiber of its own
      expect((sched.drive(command) as { value: VmValue }).value).toBe('typed');
      expect((await pending).value).toBe('carried on');
      expect(sched.stops).toEqual([]);
    });

    it('cancelling a run releases a program that was stopped', async () => {
      const { vm, sched } = make({ perform: () => null });
      const fiber = vm.addFiber([suspend(breakAt('main', 1, 'setstep')), done()]);
      const pending = (sched.drive(fiber) as Promise<unknown>).catch((e: unknown) => e);
      expect(sched.lastStopped()).toBe(fiber);

      sched.cancelAll();
      await expect(pending).resolves.toBeInstanceOf(Cancelled);
      expect(sched.stops).toEqual([]);
      expect(vm.resumed.get(fiber)).toEqual([]);
    });
  });
});
