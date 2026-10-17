// @test RF-46
import { describe, expect, it } from 'vitest';
import { beginDrag } from './drag';

describe('gestos do player no celular', () => {
  it('aceita distância no sentido certo e recusa abaixo do limiar', () => {
    const drag = beginDrag(0, 0);
    drag.move(34, 100);
    expect(drag.finish(100, 1, 0.35)).toBe(false);
    drag.move(35, 200);
    expect(drag.finish(100, 1, 0.35)).toBe(true);
    expect(drag.finish(100, -1, 0.35)).toBe(false);
  });
  it('aceita gesto rápido e rejeita cancelamento', () => {
    const drag = beginDrag(0, 0);
    drag.move(10, 10);
    expect(drag.finish(100, 1, 0.35)).toBe(true);
    expect(drag.finish(100, 1, 0.35, 100)).toBe(false);
    drag.cancel();
    expect(drag.finish(100, 1, 0.35)).toBe(false);
  });
  it('reconhece o deslizar para a esquerda', () => {
    const drag = beginDrag(100, 0);
    drag.move(64, 200);
    expect(drag.finish(100, -1, 0.35)).toBe(true);
    expect(drag.finish(100, 1, 0.35)).toBe(false);
  });
});
