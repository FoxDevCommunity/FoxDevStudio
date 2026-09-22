import { describe, expect, it } from 'vitest';
import { StringStream } from '@codemirror/language';
import { foxproParser } from '@renderer/editor/foxproMode';

function tokens(line: string): [string, string | null][] {
  const stream = new StringStream(line, 2, 2);
  const out: [string, string | null][] = [];
  const state = {} as Record<string, never>;
  while (!stream.eol()) {
    stream.start = stream.pos;
    const style = foxproParser.token(stream, state);
    const text = stream.current();
    if (text.trim()) out.push([text, style]);
  }
  return out;
}

describe('foxpro placeholder tokenizer', () => {
  it('recognises comments in all three VFP forms', () => {
    expect(tokens('* whole line')).toEqual([['* whole line', 'comment']]);
    expect(tokens('   NOTE also a comment')).toEqual([['   NOTE also a comment', 'comment']]);
    expect(tokens('x = 1 && trailing')).toEqual([
      ['x', 'variableName'],
      ['=', 'operator'],
      ['1', 'number'],
      ['&& trailing', 'comment'],
    ]);
  });
  it('recognises strings, numbers, atoms, keywords, functions and object refs case-insensitively', () => {
    expect(tokens(`IF .t. AND [x] <> "y"`)).toEqual([
      ['IF', 'keyword'],
      ['.t.', 'atom'],
      ['AND', 'keyword'],
      ['[x]', 'string'],
      ['<>', 'operator'],
      ['"y"', 'string'],
    ]);
    expect(tokens("thisform.txtName.Value = alltrim('a') + STR(1.5)")).toEqual([
      ['thisform', 'variableName.special'],
      ['.', 'operator'],
      ['txtName', 'variableName'],
      ['.', 'operator'],
      ['Value', 'variableName'],
      ['=', 'operator'],
      ['alltrim', 'typeName'],
      ['(', 'operator'],
      ["'a'", 'string'],
      [')', 'operator'],
      ['+', 'operator'],
      ['STR', 'typeName'],
      ['(', 'operator'],
      ['1.5', 'number'],
      [')', 'operator'],
    ]);
    expect(tokens('#DEFINE MAX 10')).toEqual([
      ['#DEFINE', 'meta'],
      ['MAX', 'variableName'],
      ['10', 'number'],
    ]);
    expect(tokens('d = {^2024-01-31}')).toEqual([
      ['d', 'variableName'],
      ['=', 'operator'],
      ['{^2024-01-31}', 'number'],
    ]);
  });
});
