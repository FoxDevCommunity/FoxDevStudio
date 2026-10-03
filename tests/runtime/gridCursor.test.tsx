import { act, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import userEvent from '@testing-library/user-event';
import { Desktop } from '@shared/runtime/objectModel';
import { selectGridCell } from '@renderer/runtime/gridNavigation';
import { RuntimeControl } from '@renderer/runtime/RuntimeControl';
import { useSessionStore } from '@renderer/runtime/session';
import type { WasmVm } from '@renderer/runtime/vmBridge';
import { renderWithProviders } from '../helpers/render';

const grid = () => new Desktop().instantiate({name: 'Form1', props: {Visible: true}, methods: {}, children: [{
  id: 'grid', type: 'Grid', name: 'Choices', props: {RecordSource: 'choices', RecordSourceType: 1, Height: 180}, methods: {},
  children: [{id: 'col', type: 'Column', name: 'Column1', props: {}, methods: {}, children: [
    {id: 'head', type: 'Header', name: 'Header1', props: {Caption: 'Name'}, methods: {}},
    {id: 'text', type: 'TextBox', name: 'Text1', props: {}, methods: {}},
  ]}],
}]}, -1).children[0]!;

afterEach(() => useSessionStore.setState({vm: null}));

it('shows cursor values instead of empty placeholder cells', async () => {
  const vm = {gridPreview: () => ({columns: ['NAME'], rows: [['Oak'], ['Birch']], records: [1, 2], currentRecord: 1, generation: '1', truncated: false})} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting'});
  renderWithProviders(<RuntimeControl obj={grid()} />);
  expect(await screen.findByText('Oak')).toBeInTheDocument();
  expect(screen.getByText('Birch')).toBeInTheDocument();
});

it('refreshes the preview after a runtime revision and clears closed sources', async () => {
  let value = 'Oak';
  const vm = {gridPreview: () => value ? {columns: ['NAME'], rows: [[value]], records: [1], currentRecord: 1, generation: '1', truncated: false} : {error: 'RecordSource alias is not open'}} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting'});
  renderWithProviders(<RuntimeControl obj={grid()} />);
  await screen.findByText('Oak');
  value = 'Maple';
  await act(() => useSessionStore.setState(s => ({revision: s.revision + 1})));
  expect(await screen.findByText('Maple')).toBeInTheDocument();
  expect(screen.queryByText('Oak')).toBeNull();
  value = '';
  await act(() => useSessionStore.setState(s => ({revision: s.revision + 1})));
  expect(await screen.findByText('RecordSource alias is not open')).toBeInTheDocument();
  expect(screen.queryByText('Maple')).toBeNull();
});

it('maps explicit fields and reports expressions instead of showing the wrong field', async () => {
  const vm = {gridPreview: () => ({columns: ['NAME', 'CODE'], rows: [['Oak', 'A1']], records: [1], currentRecord: 1, generation: '1', truncated: false})} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting'});
  const obj = grid();
  obj.children[0]!.set('ControlSource', 'choices.code');
  renderWithProviders(<RuntimeControl obj={obj} />);
  expect(await screen.findByText('A1')).toBeInTheDocument();
  expect(screen.queryByText('Oak')).toBeNull();
  await act(() => { obj.children[0]!.set('ControlSource', 'UPPER(name)'); useSessionStore.setState(s => ({revision: s.revision + 1})); });
  expect(await screen.findByText('(unsupported field)')).toBeInTheDocument();
});

it('moves the cursor when a displayed record is clicked', async () => {
  let currentRecord = 1;
  const select = vi.fn((_alias: string, record: number) => { currentRecord = record; });
  const vm = {gridPreview: () => ({columns: ['NAME'], rows: [['Oak'], ['Birch']], records: [1, 2], currentRecord, generation: '1', truncated: false}), gridSelect: select} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting', scheduler: null});
  const obj = grid();
  renderWithProviders(<RuntimeControl obj={obj} />);
  await userEvent.click(await screen.findByText('Birch'));
  expect(select).toHaveBeenCalledWith('choices', 2, '1');
  expect(currentRecord).toBe(2);
  expect(obj.get('ActiveRow')).toBe(2);
});

for (const scenario of ['nodefault', 'recreated', 'disabled'] as const) {
  it(`rejects navigation when BeforeRowColChange leaves the source ${scenario}`, async () => {
    let generation = '1';
    const select = vi.fn();
    const vm = {gridPreview: () => ({columns: ['NAME'], rows: [['Oak'], ['Birch']], records: [1, 2], currentRecord: 1, generation, truncated: false}), gridSelect: select} as unknown as WasmVm;
    useSessionStore.setState({vm, status: 'waiting', scheduler: null});
    const obj = grid();
    vi.spyOn(obj.desktop, 'dispatch').mockImplementation(async (_obj, event) => {
      if (event === 'BeforeRowColChange') {
        if (scenario === 'recreated') generation = '2';
        if (scenario === 'disabled') obj.set('Enabled', false);
        return {value: null, nodefault: scenario === 'nodefault'};
      }
      return {value: null, nodefault: false};
    });
    expect(await selectGridCell(obj, 2, 1, '1')).toBe(false);
    expect(select).not.toHaveBeenCalled();
  });
}

it('dispatches old and new column events around movement and supports keyboard selection', async () => {
  let currentRecord = 1;
  const order: string[] = [];
  const vm = {gridPreview: () => ({columns: ['NAME'], rows: [['Oak'], ['Birch']], records: [1, 2], currentRecord, generation: '1', truncated: false}), gridSelect: (_alias: string, record: number) => {order.push('move'); currentRecord = record;}} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting', scheduler: null});
  const obj = grid();
  vi.spyOn(obj.desktop, 'dispatch').mockImplementation(async (_obj, event) => {order.push(event); return {value: null, nodefault: false};});
  renderWithProviders(<RuntimeControl obj={obj} />);
  await userEvent.click(await screen.findByText('Birch'));
  expect(order.indexOf('BeforeRowColChange')).toBeLessThan(order.indexOf('move'));
  expect(order.indexOf('AfterRowColChange')).toBeGreaterThan(order.indexOf('move'));
  order.length = 0;
  await userEvent.keyboard('{ArrowUp}');
  expect(currentRecord).toBe(1);
  expect(order).not.toContain('Click');
});

for (const rejected of ['Valid', 'When']) {
  it(`keeps the prior record when ${rejected} rejects navigation`, async () => {
    let currentRecord = 1;
    const vm = {gridPreview: () => ({columns: ['NAME'], rows: [['Oak'], ['Birch']], records: [1, 2], currentRecord, generation: '1', truncated: false}), gridSelect: (_alias: string, record: number) => {currentRecord = record;}} as unknown as WasmVm;
    useSessionStore.setState({vm, status: 'waiting', scheduler: null});
    const obj = grid();
    const dispatch = vi.spyOn(obj.desktop, 'dispatch').mockImplementation(async (_obj, event) => ({value: event === rejected ? false : null, nodefault: false}));
    expect(await selectGridCell(obj, 2, 1, '1')).toBe(false);
    expect(currentRecord).toBe(1);
    expect(dispatch.mock.calls.some(call => call[1] === 'AfterRowColChange')).toBe(false);
  });
}
