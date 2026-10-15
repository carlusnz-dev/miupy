import { describe, expect, it } from 'vitest';
import { listboxKey } from './listbox';

const labels = ['Adicionadas', 'Título', 'Artista', 'Álbum', 'Duração'];

describe('teclado do dropdown', () => {
  // @test RNF-04
  it('setas param nas pontas; Home e End vão ao primeiro e ao último', () => {
    expect(listboxKey('ArrowDown', 1, labels)).toBe(2);
    expect(listboxKey('ArrowDown', 4, labels)).toBe(4);
    expect(listboxKey('ArrowUp', 0, labels)).toBe(0);
    expect(listboxKey('ArrowUp', 3, labels)).toBe(2);
    expect(listboxKey('Home', 3, labels)).toBe(0);
    expect(listboxKey('End', 0, labels)).toBe(4);
  });

  // @test RNF-04
  it('Enter e Espaço confirmam; outras teclas não são da lista', () => {
    expect(listboxKey('Enter', 2, labels)).toBe('commit');
    expect(listboxKey(' ', 2, labels)).toBe('commit');
    expect(listboxKey('Shift', 2, labels)).toBeNull();
    expect(listboxKey('ArrowDown', 0, [])).toBeNull();
  });

  // @test RNF-04
  it('a inicial pula para a próxima opção, ignora acento e dá a volta', () => {
    expect(listboxKey('a', 0, labels)).toBe(2);
    expect(listboxKey('a', 2, labels)).toBe(3);
    expect(listboxKey('a', 3, labels)).toBe(0);
    expect(listboxKey('d', 0, labels)).toBe(4);
    expect(listboxKey('T', 0, labels)).toBe(1);
    expect(listboxKey('z', 0, labels)).toBeNull();
  });
});
