import { StreamLanguage, type StreamParser } from '@codemirror/language';
import { FOXPRO_FUNCTIONS, FOXPRO_KEYWORDS, FOXPRO_OBJECT_REFS } from '@shared/language/foxproKeywords';

const KEYWORDS = new Set(FOXPRO_KEYWORDS);
const FUNCTIONS = new Set(FOXPRO_FUNCTIONS);
const OBJECT_REFS = new Set(FOXPRO_OBJECT_REFS);

/**
 * Placeholder FoxPro tokenizer for milestone 1: comments, strings, numbers, dot-operators,
 * keywords, built-in functions and THIS/THISFORM. Milestone 2 replaces it with a real grammar.
 */
export const foxproParser: StreamParser<Record<string, never>> = {
  name: 'foxpro',
  token(stream) {
    // line comments: * or NOTE at the start of the line, && anywhere
    if (stream.sol() && stream.match(/^\s*(\*|NOTE\b)/i)) {
      stream.skipToEnd();
      return 'comment';
    }
    if (stream.match('&&')) {
      stream.skipToEnd();
      return 'comment';
    }
    if (stream.eatSpace()) return null;
    if (stream.sol() && stream.match(/^#\w+/)) return 'meta';
    if (stream.match(/^\.(T|F|NULL|AND|OR|NOT)\./i)) return 'atom';
    if (stream.match(/^"[^"]*"?/) || stream.match(/^'[^']*'?/) || stream.match(/^\[[^\]]*\]?/)) return 'string';
    if (stream.match(/^\d+(\.\d+)?/)) return 'number';
    if (stream.match(/^\{[^}]*\}?/)) return 'number'; // date literals {^2024-01-01}
    if (stream.match(/^[A-Za-z_][A-Za-z0-9_]*/)) {
      const word = stream.current().toUpperCase();
      if (OBJECT_REFS.has(word)) return 'variableName.special';
      if (KEYWORDS.has(word)) return 'keyword';
      if (FUNCTIONS.has(word) && stream.peek() === '(') return 'typeName';
      return 'variableName';
    }
    if (stream.match(/^(==|!=|<>|<=|>=|[-+*/^=<>!$%?@&,;:().])/)) return 'operator';
    stream.next();
    return null;
  },
  languageData: { commentTokens: { line: '&&' } },
};

export const foxproLanguage = StreamLanguage.define(foxproParser);
