/**
 * Drives VM fibers and performs the side effects they yield.
 *
 * The VM is never on the JS stack while a request is being performed, so `perform` may itself
 * dispatch nested FoxPro events (ProgrammaticChange from a property set, GotFocus from
 * SetFocus, Load/Init while a form is created) and those run to completion before the outer
 * fiber resumes. That is VFP's nested-handler model without threads.
 *
 * Only one fiber ever *executes*; any number may be *parked* on a promise (a modal dialog) or
 * in READ EVENTS. A DOM event arriving while another fiber is parked starts a new fiber.
 */

import { HostError, type BreakStop, type HostRequestHandler, type RuntimeError, type StackEntry, type StepMode, type VmLike } from './host';
import type { VmValue } from './values';

export type ErrorAction = 'cancel' | 'ignore';

export interface SchedulerHooks {
  /** Unhandled runtime error. 'cancel' aborts everything, 'ignore' resumes at the next statement. */
  onError(error: RuntimeError, stack: StackEntry[]): Promise<ErrorAction> | ErrorAction;
  /** The last fiber finished and nothing is parked. */
  onIdle?(): void;
  /** QUIT, or the main program returned with no outstanding READ EVENTS. */
  onQuit?(): void;
  /** Reports status changes for the IDE status bar. */
  onStateChange?(state: SchedulerState): void;
  /**
   * A program stopped in the debugger. It stays parked - so the IDE is live while it waits -
   * until `letGo` is called for that fiber, or the session is cancelled.
   */
  onBreak?(stop: BreakStop): void;
  /** That program has been let go, so the debugger's panels have nothing to show. */
  onResumed?(fiber: number): void;
}

export type SchedulerState = 'idle' | 'running' | 'waiting';

export interface EventOutcome {
  value: VmValue;
  /** The handler executed NODEFAULT: the caller must skip the built-in behaviour. */
  nodefault: boolean;
}

interface ParkedRead {
  fiber: number;
  resolve(): void;
}

/**
 * The host waking the runtime with a payload.
 *
 * Everything else the host does either answers at once or answers a request the VM made. A
 * server is neither: it speaks first. A host event names a lambda the program handed over and
 * the arguments to call it with, and it is dispatched exactly as a Click is - a fiber of its
 * own, driven to completion. See docs/foxscript.md.
 */
export interface HostEvent {
  /** The function value to run, as the id it crossed the bridge as. */
  func: number;
  args?: VmValue[];
}

interface QueuedEvent {
  event: HostEvent;
  settle(outcome: EventOutcome | Promise<EventOutcome> | null): void;
  fail(err: unknown): void;
}

/** Ends a fiber's drive loop when the session is cancelled. */
class Cancelled extends Error {
  constructor() {
    super('Program cancelled');
    this.name = 'Cancelled';
  }
}

export class Scheduler {
  /** Bumped by `cancelAll`; parked promises check it before resuming a dead VM. */
  generation = 0;
  private executing = 0;
  private readStack: ParkedRead[] = [];
  private clearRequested = false;
  private quitting = false;
  private state: SchedulerState = 'idle';
  /** Fibers currently parked on a promise or in READ EVENTS. */
  private parked = new Set<number>();
  /** Programs stopped in the debugger: where each stopped, and what releases it. */
  private stopped = new Map<number, { stop: BreakStop; release(): void }>();
  /** Host events that arrived while a fiber was on the JS stack, oldest first. */
  private eventQueue: QueuedEvent[] = [];

  constructor(
    private readonly vm: VmLike,
    private readonly handler: HostRequestHandler,
    private readonly hooks: SchedulerHooks,
  ) {}

  get currentState(): SchedulerState {
    return this.state;
  }

  /** True while a fiber is on the JS stack; timers must queue instead of dispatching. */
  get isExecuting(): boolean {
    return this.executing > 0;
  }

  get readEventsDepth(): number {
    return this.readStack.length;
  }

  private setState(next: SchedulerState): void {
    if (this.state === next) return;
    this.state = next;
    this.hooks.onStateChange?.(next);
  }

  /**
   * Runs a fiber to completion. Returns synchronously when every request was synchronous,
   * which keeps simple event handlers (the common case) free of promise scheduling.
   */
  drive(fiber: number): EventOutcome | Promise<EventOutcome> {
    return this.runSegment(fiber, this.generation);
  }

  /**
   * One synchronous run of the pump, bracketed by enter/exit. When the pump parks, `park` has
   * already closed this segment and owns the next one, so the bracket is never double-counted.
   */
  private runSegment(fiber: number, generation: number): EventOutcome | Promise<EventOutcome> {
    this.enter();
    let result: EventOutcome | Promise<EventOutcome>;
    try {
      result = this.loop(fiber, generation);
    } catch (err) {
      this.exit();
      throw err;
    }
    if (result instanceof Promise) return result;
    this.exit();
    return result;
  }

  private enter(): void {
    this.executing++;
    this.setState('running');
  }

  private exit(): void {
    this.executing--;
    if (this.executing === 0) this.settleState();
  }

  /**
   * Once nothing is executing, an outstanding CLEAR EVENTS releases the innermost parked
   * READ EVENTS: VFP resumes the waiting program only after the handler that asked returns.
   */
  private settleState(): void {
    if (this.executing > 0) return;
    this.drainClearEvents();
    this.drainEvents();
    if (this.executing > 0) return;
    if (this.parked.size > 0) {
      this.setState('waiting');
    } else {
      this.setState('idle');
      this.hooks.onIdle?.();
    }
  }

  /** The interpreter pump. Synchronous until a request returns a promise. */
  private loop(fiber: number, generation: number): EventOutcome | Promise<EventOutcome> {
    for (;;) {
      if (generation !== this.generation) throw new Cancelled();
      const step = this.vm.step(fiber);

      if (step.state === 'done') {
        this.maybeQuitOnMainDone();
        return { value: step.value, nodefault: step.nodefault };
      }

      if (step.state === 'error') {
        const handled = this.handleError(fiber, generation, step.error, step.stack);
        if (handled instanceof Promise) return handled;
        continue;
      }

      const request = step.request;
      if (request.kind === 'Break') {
        const stop: BreakStop = { fiber, program: request.program, line: request.line, reason: request.reason };
        return this.park(fiber, generation, this.parkAtBreak(stop));
      }
      // RESUME is typed while some other program is stopped, so it is that one it lets go
      if (request.kind === 'DebugResume') {
        this.letGo(this.lastStopped());
        this.vm.resume(fiber, null);
        continue;
      }
      if (request.kind === 'ReadEvents') {
        return this.park(fiber, generation, this.parkForReadEvents(fiber));
      }
      if (request.kind === 'ClearEvents') {
        this.clearRequested = true;
        this.vm.resume(fiber, null);
        continue;
      }
      if (request.kind === 'Quit' || request.kind === 'Cancel') {
        this.quitting = request.kind === 'Quit';
        this.cancelAll();
        throw new Cancelled();
      }

      let answer: VmValue | Promise<VmValue>;
      try {
        answer = this.handler.perform(request, { fiber, generation });
      } catch (err) {
        this.injectError(fiber, err);
        continue;
      }

      if (answer instanceof Promise) {
        return this.park(
          fiber,
          generation,
          answer.then(
            (value) => {
              if (generation === this.generation) this.vm.resume(fiber, value);
            },
            (err: unknown) => {
              if (generation === this.generation) this.injectError(fiber, err);
            },
          ),
        );
      }
      this.vm.resume(fiber, answer);
    }
  }

  /** Closes the current segment, waits, then opens a new one on the same fiber. */
  private async park(fiber: number, generation: number, wait: Promise<void>): Promise<EventOutcome> {
    this.parked.add(fiber);
    this.exit();
    try {
      await wait;
    } finally {
      this.parked.delete(fiber);
    }
    if (generation !== this.generation) throw new Cancelled();
    return this.runSegment(fiber, generation);
  }

  /**
   * A breakpoint parks the fiber until the developer continues or steps.
   *
   * Nothing else about the session changes: other fibers still run, so the Command Window,
   * the forms already on screen and the debugger's own reads all work while it waits. That is
   * what a stopped program is here - one parked fiber, not a stopped world.
   */
  private parkAtBreak(stop: BreakStop): Promise<void> {
    return new Promise<void>((resolve) => {
      this.stopped.set(stop.fiber, { stop, release: resolve });
      this.setState('waiting');
      this.hooks.onBreak?.(stop);
    });
  }

  /** Where each stopped program is, in the order they stopped. */
  get stops(): BreakStop[] {
    return [...this.stopped.values()].map((s) => s.stop);
  }

  /** The program the developer would mean by "the one that is stopped": the last to stop. */
  lastStopped(): number | null {
    let last: number | null = null;
    for (const fiber of this.stopped.keys()) last = fiber;
    return last;
  }

  /**
   * Lets a stopped program go. `mode` says how far it runs before it stops again - `'go'` until
   * the next breakpoint, the rest one statement's worth. Does nothing for a fiber that is not
   * stopped, which is what a stale button click is.
   */
  letGo(fiber: number | null, mode: StepMode = 'go'): void {
    if (fiber === null) return;
    const waiting = this.stopped.get(fiber);
    if (!waiting) return;
    this.stopped.delete(fiber);
    this.vm.setStepMode?.(fiber, mode);
    this.vm.resume(fiber, null);
    this.hooks.onResumed?.(fiber);
    waiting.release();
  }

  /**
   * READ EVENTS parks the fiber until CLEAR EVENTS. VFP resumes the parked program only after
   * the handler that called CLEAR EVENTS returns, so the flag is honoured on the next drain.
   */
  private parkForReadEvents(fiber: number): Promise<void> {
    return new Promise<void>((resolve) => {
      this.readStack.push({ fiber, resolve });
      this.setState('waiting');
      // CLEAR EVENTS may already be pending (a handler asked before this program parked)
      if (this.clearRequested) queueMicrotask(() => this.settleState());
    });
  }

  /** Releases the innermost parked READ EVENTS when CLEAR EVENTS has run. */
  private drainClearEvents(): void {
    if (!this.clearRequested || this.executing > 0) return;
    const parked = this.readStack.pop();
    if (!parked) return; // CLEAR EVENTS before any READ EVENTS: stays pending, as VFP does
    this.clearRequested = false;
    this.vm.resume(parked.fiber, null);
    parked.resolve();
  }

  private maybeQuitOnMainDone(): void {
    if (this.readStack.length === 0 && this.parked.size === 0 && this.executing <= 1 && this.quitting) {
      this.hooks.onQuit?.();
    }
  }

  private injectError(fiber: number, err: unknown): void {
    if (err instanceof HostError) {
      this.vm.resumeError(fiber, err.code, err.message);
    } else {
      this.vm.resumeError(fiber, 1098, err instanceof Error ? err.message : String(err));
    }
  }

  private handleError(fiber: number, generation: number, error: RuntimeError, stack: StackEntry[]): void | Promise<EventOutcome> {
    const action = this.hooks.onError(error, stack);
    const apply = (choice: ErrorAction): void => {
      if (generation !== this.generation) return;
      if (choice === 'ignore') {
        this.vm.resume(fiber, null);
      } else {
        this.cancelAll();
      }
    };
    if (action instanceof Promise) {
      return this.park(
        fiber,
        generation,
        action.then(apply),
      ).catch((e: unknown) => {
        if (e instanceof Cancelled) return { value: null, nodefault: false };
        throw e;
      });
    }
    apply(action);
    if (action === 'cancel') throw new Cancelled();
  }

  /**
   * The host waking the runtime: runs the lambda the event names, as a fiber of its own.
   *
   * An event that arrives while a fiber is on the JS stack is queued rather than dispatched,
   * because starting one there would be a re-entrant call into the wasm exports and the bridge
   * refuses that by design. The queue drains the moment the stack unwinds, in arrival order.
   * A handler runs to completion or parks, so two events never interleave inside the VM and a
   * handler that blocks holds the queue; that is the scheduler's existing bargain and this does
   * not change it.
   *
   * Answers with what the handler returned, or `null` when the VM has no function of that id -
   * a handler left over from a run that has been cancelled.
   */
  raise(event: HostEvent): Promise<EventOutcome | null> {
    if (this.isExecuting) {
      return new Promise<EventOutcome | null>((settle, fail) => {
        this.eventQueue.push({ event, settle, fail });
      });
    }
    return Promise.resolve(this.dispatchEvent(event));
  }

  /** How many host events are waiting for the stack to unwind. */
  get queuedEvents(): number {
    return this.eventQueue.length;
  }

  private dispatchEvent(event: HostEvent): EventOutcome | Promise<EventOutcome> | null {
    const fiber = this.vm.startFunction(event.func, event.args ?? []);
    return fiber === null ? null : this.drive(fiber);
  }

  /**
   * Runs the host events that arrived while a fiber was executing, in arrival order. Called
   * once nothing is on the stack; a handler that parks closes the stack again, so the rest of
   * the queue waits for it rather than piling onto it.
   */
  private drainEvents(): void {
    while (this.eventQueue.length > 0 && this.executing === 0) {
      const queued = this.eventQueue.shift();
      if (!queued) return;
      try {
        queued.settle(this.dispatchEvent(queued.event));
      } catch (err) {
        queued.fail(err);
      }
    }
  }

  /** Starts a form/control event handler. `null` when the object has no code for it. */
  dispatch(module: number, objPath: string, event: string, thisHandle: number, args: VmValue[] = []): EventOutcome | Promise<EventOutcome> | null {
    const fiber = this.vm.startMethod(module, objPath, event, thisHandle, args);
    return fiber === null ? null : this.drive(fiber);
  }

  /** The same, for an object created from a `DEFINE CLASS` definition. */
  dispatchClass(module: number, className: string, objPath: string, event: string, thisHandle: number, args: VmValue[] = []): EventOutcome | Promise<EventOutcome> | null {
    const fiber = this.vm.startClassMethod(module, className, objPath, event, thisHandle, args);
    return fiber === null ? null : this.drive(fiber);
  }

  /** Runs a program module's main body. */
  runProgram(module: number, funcName = 'MAIN', args: VmValue[] = []): EventOutcome | Promise<EventOutcome> {
    return this.drive(this.vm.start(module, funcName, null, args));
  }

  /** Aborts every fiber; parked promises see the new generation and stop. */
  cancelAll(): void {
    this.generation++;
    this.readStack = [];
    this.clearRequested = false;
    this.parked.clear();
    // a queued host event has no runtime left to run in; whoever raised it is told nothing ran
    const queued = this.eventQueue;
    this.eventQueue = [];
    for (const q of queued) q.settle(null);
    // a program stopped in the debugger is cancelled like any other: its drive loop is released
    // so the promise does not outlive the run, and sees the new generation rather than resuming
    const stopped = [...this.stopped.values()];
    this.stopped.clear();
    for (const waiting of stopped) {
      this.hooks.onResumed?.(waiting.stop.fiber);
      waiting.release();
    }
    this.vm.abortAll();
    this.setState('idle');
  }
}

export { Cancelled };
