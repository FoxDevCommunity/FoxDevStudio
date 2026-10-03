import { act, screen } from '@testing-library/react';
import { afterEach, expect, it } from 'vitest';
import { Desktop } from '@shared/runtime/objectModel';
import { RuntimeControl } from '@renderer/runtime/RuntimeControl';
import { useSessionStore } from '@renderer/runtime/session';
import type { WasmVm } from '@renderer/runtime/vmBridge';
import { renderWithProviders } from '../helpers/render';

const grid = () => new Desktop().instantiate({name: 'Form1', props: {}, methods: {}, children: [{
  id: 'grid', type: 'Grid', name: 'Choices', props: {RecordSource: 'choices', RecordSourceType: 1, Height: 180}, methods: {},
  children: [{id: 'col', type: 'Column', name: 'Column1', props: {}, methods: {}, children: [
    {id: 'head', type: 'Header', name: 'Header1', props: {Caption: 'Name'}, methods: {}},
  ]}],
}]}, -1).children[0]!;

afterEach(() => useSessionStore.setState({vm: null}));

it('shows cursor values instead of empty placeholder cells', async () => {
  const vm = {gridPreview: () => ({columns: ['NAME'], rows: [['Oak'], ['Birch']], truncated: false})} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting'});
  renderWithProviders(<RuntimeControl obj={grid()} />);
  expect(await screen.findByText('Oak')).toBeInTheDocument();
  expect(screen.getByText('Birch')).toBeInTheDocument();
});

it('refreshes the preview after a runtime revision and clears closed sources', async () => {
  let value = 'Oak';
  const vm = {gridPreview: () => value ? {columns: ['NAME'], rows: [[value]], truncated: false} : {error: 'RecordSource alias is not open'}} as unknown as WasmVm;
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
  const vm = {gridPreview: () => ({columns: ['NAME', 'CODE'], rows: [['Oak', 'A1']], truncated: false})} as unknown as WasmVm;
  useSessionStore.setState({vm, status: 'waiting'});
  const obj = grid();
  obj.children[0]!.set('ControlSource', 'choices.code');
  renderWithProviders(<RuntimeControl obj={obj} />);
  expect(await screen.findByText('A1')).toBeInTheDocument();
  expect(screen.queryByText('Oak')).toBeNull();
  await act(() => { obj.children[0]!.set('ControlSource', 'UPPER(name)'); useSessionStore.setState(s => ({revision: s.revision + 1})); });
  expect(await screen.findByText('(unsupported field)')).toBeInTheDocument();
});
