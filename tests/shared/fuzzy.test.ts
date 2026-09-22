import { describe, expect, it } from 'vitest';
import { fuzzyFilter, fuzzyMatch } from '@shared/fuzzy';

const order = (query: string, items: string[]) => fuzzyFilter(query, items, (s) => s).map((r) => r.item);

describe('fuzzy matching', () => {
  it('matches a subsequence, not a substring', () => {
    expect(fuzzyMatch('pgf', 'controls/pgframe/msgbox.fxf')).not.toBeNull();
    expect(fuzzyMatch('cpm', 'controls/pgframe/msgbox.fxf')).not.toBeNull();
    expect(fuzzyMatch('zz', 'controls/pgframe/msgbox.fxf')).toBeNull();
  });

  it('is case insensitive and reports where it matched', () => {
    const m = fuzzyMatch('FXF', 'order.fxf');
    expect(m?.positions).toEqual([6, 7, 8]);
  });

  it('matches everything when the query is empty, so a search box can start blank', () => {
    expect(fuzzyMatch('', 'anything')).toEqual({ score: 0, positions: [] });
    expect(order('', ['b', 'a'])).toEqual(['b', 'a']);
  });

  it('puts a run of adjacent characters above scattered ones', () => {
    expect(order('order', ['o_r_d_e_r.prg', 'order.prg'])[0]).toBe('order.prg');
  });

  it('prefers a match at the start of a word', () => {
    // `db` at a path separator beats `db` inside a longer word
    expect(order('db', ['adbcadbc.prg', 'db/order.fxf'])[0]).toBe('db/order.fxf');
  });

  it('prefers an earlier match when nothing else separates two candidates', () => {
    expect(order('form', ['a/b/c/form.fxf', 'form.fxf'])[0]).toBe('form.fxf');
  });

  it('keeps the original order among equals', () => {
    expect(order('x', ['x1', 'x2', 'x3'])).toEqual(['x1', 'x2', 'x3']);
  });

  it('drops what does not match at all', () => {
    expect(order('qq', ['form.fxf', 'menu.fxm'])).toEqual([]);
  });
});
