import { beforeEach, describe, expect, it, vi } from 'vitest';
import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { BreakStop, DebugFrame, DebugVariable, StepMode } from '@shared/runtime/host';
import type { VmValue } from '@shared/runtime/values';
import { DebuggerPanel } from '@renderer/runtime/DebuggerPanel';
import { useDebugStore, type DebugVm } from '@renderer/runtime/debugSession';
import { renderWithProviders } from '../helpers/render';

/**
 * A VM stopped in `Twice`, called from `counting`. The reads are the synchronous ones the real
 * VM answers while the fiber is parked, so the panel here does exactly what it does in the IDE.
 */
class StoppedVm implements DebugVm {
  readonly breakpoints: { program: string; line: number; on: boolean }[] = [];
  setBreakpoint(program: string, line: number, on: boolean): void {
    this.breakpoints.push({ program, line, on });
  }
  frames(): DebugFrame[] {
    return [
      { program: 'counting', module: 'counting', line: 4 },
      { program: 'Twice', module: 'counting', line: 11 },
    ];
  }
  frameVariables(_fiber: number, level: number): DebugVariable[] {
    return level === 1
      ? [{ name: 'N', private: false, value: 3 }]
      : [
          { name: 'NTOTAL', private: false, value: 6 },
          { name: 'PCLABEL', private: true, value: 'twice' },
        ];
  }
  evaluateIn(_fiber: number, level: number, expr: string): VmValue {
    if (expr === 'nope') return { code: 12, message: "Variable 'NOPE' is not found", program: 'Twice', line: 11 } as unknown as VmValue;
    return level === 1 ? 3 : 6;
  }
}

const stop: BreakStop = { fiber: 7, program: 'Twice', line: 11, reason: 'breakpoint' };

function attach(): { vm: StoppedVm; letGo: ReturnType<typeof vi.fn> } {
  const vm = new StoppedVm();
  const letGo = vi.fn<(fiber: number | null, mode?: StepMode) => void>();
  useDebugStore.getState().attach(vm, { letGo });
  return { vm, letGo };
}

describe('the debugger panel', () => {
  beforeEach(() => {
    useDebugStore.setState({ stop: null, frames: [], level: 0, locals: [], watches: [], breakpoints: [], attached: false });
    useDebugStore.getState().detach();
  });

  it('shows where the program stopped, its call stack and the innermost frame', () => {
    attach();
    useDebugStore.getState().stopped(stop);
    renderWithProviders(<DebuggerPanel />);

    expect(screen.getByText(/Breakpoint in Twice, line 11/)).toBeTruthy();
    const stack = within(screen.getByLabelText('Call stack')).getAllByRole('button');
    expect(stack.map((b) => b.textContent)).toEqual(['counting (4)', 'Twice (11)']);
    // the innermost frame is the one a developer means, so it is the one that is read
    expect(within(screen.getByLabelText('Locals')).getByText(/^N = 3$/)).toBeTruthy();
  });

  it('reads Locals and the watches against whichever frame is chosen', async () => {
    const user = userEvent.setup();
    attach();
    useDebugStore.getState().stopped(stop);
    useDebugStore.getState().addWatch('nTotal');
    renderWithProviders(<DebuggerPanel />);

    await user.click(within(screen.getByLabelText('Call stack')).getByText('counting (4)'));
    const locals = within(screen.getByLabelText('Locals'));
    expect(locals.getByText(/NTOTAL = 6/)).toBeTruthy();
    expect(locals.getByText(/PCLABEL \(private\) = twice/)).toBeTruthy();
    expect(within(screen.getByLabelText('Watch expressions')).getByText(/nTotal = 6/)).toBeTruthy();
  });

  it('takes a watch expression and says when it cannot be read from here', async () => {
    const user = userEvent.setup();
    attach();
    useDebugStore.getState().stopped(stop);
    renderWithProviders(<DebuggerPanel />);

    await user.type(screen.getByLabelText('Watch expression'), 'nope');
    await user.click(screen.getByRole('button', { name: 'Watch' }));
    expect(within(screen.getByLabelText('Watch expressions')).getByText(/Variable 'NOPE' is not found/)).toBeTruthy();
  });

  it('continues and steps the stopped program', async () => {
    const user = userEvent.setup();
    const { letGo } = attach();
    useDebugStore.getState().stopped(stop);
    renderWithProviders(<DebuggerPanel />);

    await user.click(screen.getByText('Step Over'));
    await user.click(screen.getByText('Continue'));
    expect(letGo.mock.calls).toEqual([
      [7, 'over'],
      [7, 'go'],
    ]);
  });

  it('arms the breakpoints it kept in the VM of the next run, and lists them', () => {
    useDebugStore.getState().toggleBreakpoint('counting', 4);
    useDebugStore.getState().toggleBreakpoint('cmdSayHi.Click', 2);
    const { vm } = attach();
    expect(vm.breakpoints).toEqual([
      { program: 'counting', line: 4, on: true },
      { program: 'cmdSayHi.Click', line: 2, on: true },
    ]);
    // an editor asks for the lines of the source it is showing, whatever their case
    expect(useDebugStore.getState().linesIn('COUNTING')).toEqual([4]);

    renderWithProviders(<DebuggerPanel />);
    const listed = within(screen.getByLabelText('Breakpoints')).getAllByRole('button');
    expect(listed.map((b) => b.textContent)).toEqual(['counting (4)', 'cmdSayHi.Click (2)']);
  });

  it('a second click on a breakpoint takes it away', () => {
    const { vm } = attach();
    useDebugStore.getState().toggleBreakpoint('counting', 4);
    useDebugStore.getState().toggleBreakpoint('counting', 4);
    expect(useDebugStore.getState().breakpoints).toEqual([]);
    expect(vm.breakpoints.map((b) => b.on)).toEqual([true, false]);
  });
});
