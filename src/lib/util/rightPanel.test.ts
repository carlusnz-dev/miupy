import { describe, expect, it } from 'vitest';
import { parseRightPanel, rightPanelOpen, upcoming } from './rightPanel';

describe('painel da direita', () => {
  // @test RF-39
  it('lê a escolha salva e cai no padrão pela largura', () => {
    expect(parseRightPanel('1')).toBe(true);
    expect(parseRightPanel('0')).toBe(false);
    expect(parseRightPanel(null)).toBeNull();
    expect(parseRightPanel('x')).toBeNull();
    expect(rightPanelOpen(null, 1280)).toBe(true);
    expect(rightPanelOpen(null, 1279)).toBe(false);
    expect(rightPanelOpen(false, 1920)).toBe(false);
    expect(rightPanelOpen(true, 900)).toBe(true);
  });

  // @test RF-39
  it('lista a faixa atual e as próximas com o índice da fila', () => {
    expect(upcoming(['a', 'b', 'c'], 1)).toEqual([{ track: 'b', at: 1 }, { track: 'c', at: 2 }]);
    expect(upcoming(['a', 'b'], -1)).toEqual([{ track: 'a', at: 0 }, { track: 'b', at: 1 }]);
    expect(upcoming([], 0)).toEqual([]);
  });
});
