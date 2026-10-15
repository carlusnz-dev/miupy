import { describe, expect, it } from 'vitest';
import { letterDelays, splitWords } from './animatedText';

describe('animatedText', () => {
  // @test RNF-08
  it('divide em palavras sem quebrar letras acentuadas nem espaços repetidos', () => {
    expect(splitWords('  Coração  Selvagem ')).toEqual([['C', 'o', 'r', 'a', 'ç', 'ã', 'o'], ['S', 'e', 'l', 'v', 'a', 'g', 'e', 'm']]);
    expect(splitWords('')).toEqual([]);
  });

  // @test RNF-08
  it('escalona as letras em ordem e limita o total em títulos longos', () => {
    expect(letterDelays([['a', 'b'], ['c']], 20)).toEqual([[0, 20], [40]]);
    const long = letterDelays([Array.from({ length: 200 }, () => 'x')], 22, 520);
    expect(long[0][199]).toBe(520);
  });
});
