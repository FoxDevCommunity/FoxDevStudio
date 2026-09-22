/**
 * The debugger's state: where the program is stopped, what it can see from there, and the
 * breakpoints that stop it.
 *
 * Breakpoints outlive a run - they are set in the editor before anything is running - so they
 * live here and are pushed into each VM as it is created. Everything else is read from the
 * stopped fiber on demand: the VM is off the JS stack while a program waits at a breakpoint,
 * so the call stack, the variables of a frame and a watch expression are all plain synchronous
 * reads.
 */

import { create } from 'zustand';
import type { BreakStop, DebugFrame, DebugVariable, RuntimeError, StepMode } from '@shared/runtime/host';
import { displayValue, type VmValue } from '@shared/runtime/values';

/** Where a breakpoint is: the source's name as the call stack shows it, and a line in it. */
export interface Breakpoint {
  program: string;
  line: number;
}

/** One watch expression and what it last came to. */
export interface WatchRow {
  text: string;
  value: string;
  /** The expression could not be read from here - an unknown name, a syntax error. */
  failed: boolean;
}

/** What the debugger needs of the VM. Kept structural so a test can stand in for it. */
export interface DebugVm {
  setBreakpoint(program: string, line: number, on: boolean): void;
  frames(fiber: number): DebugFrame[];
  frameVariables(fiber: number, level: number): DebugVariable[];
  evaluateIn(fiber: number, level: number, expr: string): VmValue;
}

/** What the debugger needs of the scheduler: the way to let a stopped program go. */
export interface DebugRun {
  letGo(fiber: number | null, mode?: StepMode): void;
}

export interface DebugState {
  /** Where the program stopped, or null when nothing is stopped. */
  stop: BreakStop | null;
  /** The call stack of the stopped program, outermost first. */
  frames: DebugFrame[];
  /** The frame Locals and Watch read, an index into `frames`; the innermost by default. */
  level: number;
  locals: DebugVariable[];
  watches: WatchRow[];
  breakpoints: Breakpoint[];
  /** True while a run is under way, so the panel's buttons mean something. */
  attached: boolean;

  /** Binds the debugger to the run that is starting, and arms its breakpoints in that VM. */
  attach(vm: DebugVm, run: DebugRun): void;
  /** The run is over: nothing is stopped and nothing can be stepped. */
  detach(): void;
  /** The scheduler reports a program stopped. */
  stopped(stop: BreakStop): void;
  /** ...and that it has been let go. */
  resumed(): void;
  /** Reads Locals and the watches against another frame of the stopped program. */
  selectFrame(level: number): void;
  addWatch(text: string): void;
  removeWatch(index: number): void;
  /** Turns a breakpoint on where there is none and off where there is one. */
  toggleBreakpoint(program: string, line: number): void;
  clearBreakpoints(): void;
  /** Every breakpoint in one source, for an editor's gutter. */
  linesIn(program: string): number[];
  /** Lets the stopped program go: `'go'` to the next breakpoint, the rest one step. */
  letGo(mode?: StepMode): void;
}

/** The VM and scheduler of the run in hand. Not state: nothing renders from them. */
let bound: { vm: DebugVm; run: DebugRun } | null = null;

/** A watch expression read in a frame of the stopped program, as text for the panel. */
function readWatch(vm: DebugVm, fiber: number, level: number, text: string): WatchRow {
  try {
    const value = vm.evaluateIn(fiber, level, text);
    // an expression that could not be read comes back as the error rather than a value
    const error = value as Partial<RuntimeError> | null;
    if (error && typeof error === 'object' && typeof error.message === 'string' && 'code' in error) {
      return { text, value: error.message, failed: true };
    }
    return { text, value: displayValue(value), failed: false };
  } catch (e) {
    return { text, value: e instanceof Error ? e.message : String(e), failed: true };
  }
}

export const useDebugStore = create<DebugState>((set, get) => {
  /** Reads the stopped program again: its stack, the chosen frame's variables, the watches. */
  const refresh = (level?: number) => {
    const { stop, watches } = get();
    if (!bound || !stop) {
      set({ frames: [], locals: [], watches: watches.map((w) => ({ ...w, value: '', failed: false })) });
      return;
    }
    const frames = bound.vm.frames(stop.fiber);
    const at = Math.min(Math.max(level ?? frames.length - 1, 0), Math.max(frames.length - 1, 0));
    set({
      frames,
      level: at,
      locals: bound.vm.frameVariables(stop.fiber, at),
      watches: watches.map((w) => readWatch(bound!.vm, stop.fiber, at, w.text)),
    });
  };

  return {
    stop: null,
    frames: [],
    level: 0,
    locals: [],
    watches: [],
    breakpoints: [],
    attached: false,

    attach(vm, run) {
      bound = { vm, run };
      for (const bp of get().breakpoints) vm.setBreakpoint(bp.program, bp.line, true);
      set({ attached: true, stop: null, frames: [], locals: [] });
    },

    detach() {
      bound = null;
      set({ attached: false, stop: null, frames: [], locals: [], watches: get().watches.map((w) => ({ ...w, value: '', failed: false })) });
    },

    stopped(stop) {
      set({ stop });
      refresh();
    },

    resumed() {
      set({ stop: null, frames: [], locals: [] });
    },

    selectFrame(level) {
      refresh(level);
    },

    addWatch(text) {
      const trimmed = text.trim();
      if (trimmed === '') return;
      set({ watches: [...get().watches, { text: trimmed, value: '', failed: false }] });
      refresh(get().level);
    },

    removeWatch(index) {
      set({ watches: get().watches.filter((_, i) => i !== index) });
    },

    toggleBreakpoint(program, line) {
      const at = get().breakpoints.findIndex((b) => b.line === line && b.program.toUpperCase() === program.toUpperCase());
      const on = at < 0;
      set({
        breakpoints: on ? [...get().breakpoints, { program, line }] : get().breakpoints.filter((_, i) => i !== at),
      });
      bound?.vm.setBreakpoint(program, line, on);
    },

    clearBreakpoints() {
      for (const bp of get().breakpoints) bound?.vm.setBreakpoint(bp.program, bp.line, false);
      set({ breakpoints: [] });
    },

    linesIn(program) {
      const wanted = program.toUpperCase();
      return get().breakpoints.filter((b) => b.program.toUpperCase() === wanted).map((b) => b.line);
    },

    letGo(mode: StepMode = 'go') {
      const stop = get().stop;
      if (!bound || !stop) return;
      bound.run.letGo(stop.fiber, mode);
    },
  };
});

/** What the panel calls the reason a program stopped. */
export function reasonLabel(stop: BreakStop): string {
  switch (stop.reason) {
    case 'breakpoint':
      return 'Breakpoint';
    case 'step':
      return 'Stepped';
    case 'suspend':
      return 'SUSPEND';
    case 'setstep':
      return 'SET STEP ON';
  }
}
